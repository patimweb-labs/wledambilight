// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Patrick Weppelmann

//! The test picture: instead of the screen (`AMBILIGHT_DEMO=1` or the
//! "Test picture" switch in the app). It needs no capture permission, works on
//! every system, and shows at a glance whether the strip follows the picture.
//!
//! - the hue runs **left → right** through the whole rainbow, so a single
//!   horizontal zone (the normal case: one strip along one edge) shows red at
//!   LED 0 and violet at the last LED. Mirrored order = the strip runs
//!   backwards (device → *Reverse order*) or the zone points the other way.
//! - brightness falls from top to bottom, so a top zone and a bottom zone can
//!   be told apart at a glance.
//! - a white bar travels left → right once every few seconds: direction and
//!   lag of the whole chain become visible.
//! - a white frame marks the exact edges of the picture; the cross in the
//!   centre is the colour reference (pure white).

use std::time::{Duration, Instant};

use super::{frame_height, CaptureOptions, CaptureStatus, Handles};

/// `AMBILIGHT_DEMO=1`: test picture instead of the screen, without any setting.
pub fn enabled() -> bool {
    std::env::var_os("AMBILIGHT_DEMO").is_some()
}

/// The screen the test picture pretends to be (aspect and size only).
const SCREEN: (u32, u32) = (2560, 1440);

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i.rem_euclid(6) {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

#[inline]
fn put(rgb: &mut [u8], o: usize, c: [u8; 3]) {
    rgb[o] = c[0];
    rgb[o + 1] = c[1];
    rgb[o + 2] = c[2];
}

/// One frame of the test picture at `w × h`, `t` seconds after the start.
pub fn picture(w: u32, h: u32, t: f32) -> Vec<u8> {
    let (w, h) = (w.max(2), h.max(2));
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    let drift = t * 0.06; // the rainbow slides through the hues, ~17 s per lap
    let bar = (t * 0.18).rem_euclid(1.2) - 0.1; // white bar, ~5.5 s per run
    let bar_w = 0.025;
    for y in 0..h {
        // bright at the top, about 45 % at the bottom: which edge is which
        let v = 1.0 - 0.55 * (y as f32 + 0.5) / h as f32;
        for x in 0..w {
            let fx = (x as f32 + 0.5) / w as f32;
            let hue = fx + drift;
            let on_bar = (fx - bar).abs() < bar_w * 0.5;
            let c = if on_bar {
                [1.0, 1.0, 1.0]
            } else {
                hsv(hue, 0.95, v)
            };
            let o = ((y * w + x) * 3) as usize;
            put(
                &mut rgb,
                o,
                [
                    (c[0] * 255.0).clamp(0.0, 255.0) as u8,
                    (c[1] * 255.0).clamp(0.0, 255.0) as u8,
                    (c[2] * 255.0).clamp(0.0, 255.0) as u8,
                ],
            );
        }
    }
    // the white frame marks where the picture ends
    let frame = (w / 200).max(1);
    for y in 0..h {
        for x in 0..w {
            if x < frame || y < frame || x + frame >= w || y + frame >= h {
                let o = ((y * w + x) * 3) as usize;
                put(&mut rgb, o, [255, 255, 255]);
            }
        }
    }
    // a small white cross in the middle: the picture is alive
    let (mx, my) = (w / 2, h / 2);
    let arm_x = (w / 40).max(2);
    let arm_y = (h / 40).max(2);
    for y in my.saturating_sub(arm_y)..(my + arm_y).min(h) {
        for x in mx.saturating_sub(arm_x)..(mx + arm_x).min(w) {
            let on_x = x == mx || x + 1 == mx;
            let on_y = y == my || y + 1 == my;
            if on_x || on_y {
                let o = ((y * w + x) * 3) as usize;
                put(&mut rgb, o, [255, 255, 255]);
            }
        }
    }
    rgb
}

pub fn run(opts: CaptureOptions, h: Handles) {
    let (sw, sh) = SCREEN;
    h.set_screen(sw, sh);
    let mut w = opts.width.max(2);
    let mut fh = frame_height(w, sw, sh);
    h.set_status(CaptureStatus::Running { w, h: fh });
    let start = Instant::now();
    let frame_time = Duration::from_secs_f32(1.0 / opts.fps.clamp(1, 120) as f32);
    while !h.stopped() {
        // a new resolution arrives as a width change, no restart needed
        let want = h.width().max(2);
        if want != w {
            w = want;
            fh = frame_height(w, sw, sh);
            h.set_status(CaptureStatus::Running { w, h: fh });
        }
        let t = start.elapsed().as_secs_f32();
        h.publish(w, fh, picture(w, fh, t));
        std::thread::sleep(frame_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(frame: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
        let o = ((y * w + x) * 3) as usize;
        [frame[o], frame[o + 1], frame[o + 2]]
    }

    #[test]
    fn the_picture_has_the_right_size() {
        let f = picture(64, 36, 0.0);
        assert_eq!(f.len(), 64 * 36 * 3);
    }

    #[test]
    fn the_frame_is_white_on_all_four_edges() {
        let (w, h) = (64u32, 36u32);
        let f = picture(w, h, 3.0);
        for x in [0, 1, w - 1] {
            assert_eq!(px(&f, w, x, 0), [255, 255, 255], "top edge at x={x}");
            assert_eq!(px(&f, w, x, h - 1), [255, 255, 255], "bottom edge at x={x}");
        }
        for y in [0, h - 1] {
            assert_eq!(px(&f, w, 0, y), [255, 255, 255], "left edge at y={y}");
            assert_eq!(px(&f, w, w - 1, y), [255, 255, 255], "right edge at y={y}");
        }
    }

    #[test]
    fn the_picture_is_bright_and_colourful() {
        let (w, h) = (320u32, 180u32);
        let f = picture(w, h, 1.0);
        // half way along the top edge, a few pixels in
        let c = px(&f, w, w / 3, 6);
        assert!(c.iter().max().unwrap() > &180, "must be bright: {c:?}");
        let spread = *c.iter().max().unwrap() as i32 - *c.iter().min().unwrap() as i32;
        assert!(spread > 80, "must be colourful: {c:?}");
    }

    #[test]
    fn the_rainbow_drifts_over_time() {
        let (w, h) = (64u32, 36u32);
        let a = picture(w, h, 0.0);
        let b = picture(w, h, 2.0);
        let before = px(&a, w, 9, h / 2);
        let after = px(&b, w, 9, h / 2);
        assert_ne!(
            before, after,
            "after two seconds the colours must have moved"
        );
    }

    #[test]
    fn the_top_is_brighter_than_the_bottom() {
        let (w, h) = (64u32, 36u32);
        let f = picture(w, h, 0.0);
        let bright = |y: u32| {
            let mut sum = 0u32;
            for x in 4..w - 4 {
                sum += px(&f, w, x, y).iter().map(|v| *v as u32).sum::<u32>();
            }
            sum
        };
        assert!(
            bright(4) > bright(h - 5) * 3 / 2,
            "top {} vs bottom {}",
            bright(4),
            bright(h - 5)
        );
    }

    #[test]
    fn the_white_bar_travels_to_the_right() {
        let (w, h) = (200u32, 100u32);
        let row = h / 3; // away from the centre cross, which is white as well
        let bar_x = |frame: &[u8]| {
            // the bar is the only really white column: high in all channels
            (2..w - 2)
                .max_by_key(|x| px(frame, w, *x, row).iter().copied().min().unwrap())
                .unwrap()
        };
        let a = bar_x(&picture(w, h, 1.0));
        let b = bar_x(&picture(w, h, 2.0));
        assert!(b > a + 10, "bar at {a} then {b}: it must move to the right");
    }

    #[test]
    fn the_rainbow_runs_left_to_right() {
        // a quarter and three quarters are half a turn apart
        let (w, h) = (200u32, 100u32);
        let f = picture(w, h, 0.0);
        let left = px(&f, w, w / 4, h / 2);
        let right = px(&f, w, 3 * w / 4, h / 2);
        let diff = (0..3)
            .map(|k| (left[k] as i32 - right[k] as i32).abs())
            .max()
            .unwrap();
        assert!(diff > 100, "quarter {left:?} vs three quarters {right:?}");
    }

    #[test]
    fn a_new_resolution_arrives_without_a_restart() {
        let cap = super::super::ScreenCapture::start(CaptureOptions {
            width: 160,
            fps: 30,
            monitor: 0,
            restore_token: None,
            test_image: true,
        });
        let wait_for = |w: u32| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                let f = cap.frame();
                if f.w == w && f.h > 0 && f.data.len() == (f.w * f.h * 3) as usize {
                    return f;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            panic!("no frame at width {w}");
        };
        assert_eq!(wait_for(160).w, 160);
        cap.set_width(320);
        let wide = wait_for(320);
        assert_eq!(wide.w, 320);
        assert!(wide.data.iter().any(|b| *b != 0), "still a picture");
        cap.set_width(160);
        assert_eq!(wait_for(160).w, 160);
    }

    #[test]
    fn the_capture_runs_without_a_screen() {
        // the whole point of the test picture: no portal, no permission
        let mut cap = super::super::ScreenCapture::start(CaptureOptions {
            width: 160,
            fps: 30,
            monitor: 0,
            restore_token: None,
            test_image: true,
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seq = 0;
        while Instant::now() < deadline {
            let f = cap.frame();
            if f.seq > 1 && f.w == 160 && f.data.len() == (f.w * f.h * 3) as usize {
                seq = f.seq;
                assert!(f.data.iter().any(|b| *b != 0), "frame must have colour");
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(seq > 1, "test picture delivered no frames");
        assert!(matches!(cap.status(), CaptureStatus::Running { .. }));
    }
}
