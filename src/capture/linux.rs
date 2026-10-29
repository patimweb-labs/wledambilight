// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Patrick Weppelmann

//! Linux: xdg-desktop-portal ScreenCast -> PipeWire node -> `gst-launch-1.0`.
//!
//! The portal hands out a PipeWire fd and node id. GStreamer converts and
//! scales the stream to the small RGB buffer we need and writes raw frames to
//! stdout. Running GStreamer as a child process keeps the binary free of any
//! GStreamer/PipeWire build dependency and works the same in a Flatpak.

use std::io::Read;
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use ashpd::enumflags2::BitFlags;

use super::{frame_height, CaptureOptions, CaptureStatus, Handles};
use crate::i18n::tr;

struct PortalStream {
    node: u32,
    size: (i32, i32),
    token: Option<String>,
    fd: std::os::fd::OwnedFd,
    _session: ashpd::desktop::Session<Screencast>,
}

async fn open_stream(restore: Option<String>) -> Result<PortalStream, String> {
    let proxy = Screencast::new()
        .await
        .map_err(|e| format!("portal: {e}"))?;
    let session = proxy
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(|e| format!("session: {e}"))?;
    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_sources(BitFlags::from(SourceType::Monitor))
                .set_multiple(false)
                .set_cursor_mode(CursorMode::Hidden)
                .set_persist_mode(PersistMode::ExplicitlyRevoked)
                .set_restore_token(restore.as_deref()),
        )
        .await
        .map_err(|e| format!("select sources: {e}"))?;
    let request = proxy
        .start(&session, None, Default::default())
        .await
        .map_err(|e| format!("start: {e}"))?;
    let streams = request
        .response()
        .map_err(|e| format!("{}: {e}", tr("Freigabe abgelehnt", "share refused")))?;
    let first = streams
        .streams()
        .first()
        .ok_or_else(|| {
            tr("Portal lieferte keinen Stream", "portal returned no stream").to_string()
        })?
        .clone();
    let fd = proxy
        .open_pipe_wire_remote(&session, Default::default())
        .await
        .map_err(|e| format!("pipewire remote: {e}"))?;
    Ok(PortalStream {
        node: first.pipe_wire_node_id(),
        size: first.size().unwrap_or((0, 0)),
        token: streams.restore_token().map(|s| s.to_string()),
        fd,
        _session: session,
    })
}

pub fn monitors() -> Vec<String> {
    Vec::new()
}

/// Start `gst-launch-1.0` on the portal's PipeWire node. Only this process
/// depends on the frame size, so a new resolution restarts it and nothing else.
fn spawn_gst(fd: i32, node: u32, width: u32, height: u32) -> std::io::Result<Child> {
    let pipeline = format!(
        "pipewiresrc fd=3 path={node} do-timestamp=true ! videoconvert ! videoscale ! video/x-raw,format=RGB,width={width},height={height} ! fdsink fd=1"
    );
    let mut cmd = Command::new("gst-launch-1.0");
    cmd.arg("-q");
    for tok in pipeline.split_whitespace() {
        cmd.arg(tok);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(move || {
            // die with the parent instead of lingering as an orphan
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            if fd != 3 && libc::dup2(fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let flags = libc::fcntl(3, libc::F_GETFD);
            libc::fcntl(3, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            Ok(())
        });
    }
    cmd.spawn()
}

pub fn run(opts: CaptureOptions, h: Handles) {
    h.set_status(CaptureStatus::Starting);
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return h.fail(format!("runtime: {e}")),
    };
    let stream = match rt.block_on(open_stream(opts.restore_token)) {
        Ok(s) => s,
        Err(e) => return h.fail(e),
    };
    if let Some(t) = stream.token.clone() {
        *h.restore_token.lock().unwrap() = Some(t);
    }
    let (sw, sh) = (stream.size.0.max(0) as u32, stream.size.1.max(0) as u32);
    h.set_screen(sw, sh);

    let raw = stream.fd.as_raw_fd();
    let mut width = h.width().max(2);
    let mut fh = frame_height(width, sw, sh);
    let mut child = match spawn_gst(raw, stream.node, width, fh) {
        Ok(c) => c,
        Err(e) => {
            return h.fail(format!(
            "{}: {e}",
            tr(
                "gst-launch-1.0 nicht gefunden – bitte GStreamer + PipeWire-Plugin installieren",
                "gst-launch-1.0 not found – please install GStreamer and its PipeWire plugin"
            )
        ))
        }
    };
    let Some(mut stdout) = child.stdout.take() else {
        return h.fail("no gst stdout");
    };
    h.set_status(CaptureStatus::Running { w: width, h: fh });
    let mut frame_len = (width * fh * 3) as usize;

    loop {
        if h.stopped() {
            break;
        }
        // a new resolution arrives as a width change: only GStreamer is rebuilt,
        // the portal session and its PipeWire node stay alive
        let want = h.width().max(2);
        if want != width {
            width = want;
            fh = frame_height(width, sw, sh);
            frame_len = (width * fh * 3) as usize;
            let _ = child.kill();
            let _ = child.wait();
            match spawn_gst(raw, stream.node, width, fh) {
                Ok(mut c) => match c.stdout.take() {
                    Some(s) => {
                        stdout = s;
                        child = c;
                        h.set_status(CaptureStatus::Running { w: width, h: fh });
                    }
                    None => {
                        h.fail("no gst stdout");
                        break;
                    }
                },
                Err(e) => {
                    h.fail(format!("gst-launch-1.0: {e}"));
                    break;
                }
            }
            continue;
        }
        let mut buf = vec![0u8; frame_len];
        match stdout.read_exact(&mut buf) {
            Ok(()) => h.publish(width, fh, buf),
            Err(e) => {
                if !h.stopped() {
                    h.fail(format!(
                        "{}: {e}",
                        tr(
                            "Aufnahme beendet – über „Aufnahme neu starten“ wieder verbinden",
                            "capture stopped – reconnect with “Restart capture”"
                        )
                    ));
                }
                break;
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    // keep the portal session alive until the very end
    drop(stream.fd);
    drop(stream._session);
    std::thread::sleep(Duration::from_millis(10));
}
