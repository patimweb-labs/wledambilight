// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Das aufgenommene Bild als Textur für die Bühne.
//!
//! Die Vorschau ist ein Bild zum Zonen setzen, kein Videobild: mehr als
//! [`MAX_BREITE`] Pixel Breite kosten nur Zeit beim Hochladen. Verkleinert
//! wird mit dem nächsten Nachbarn – schnell, und das Bild bleibt scharf genug,
//! um Kanten zu treffen.

use wled_ambilight::capture::SharedFrame;

pub const MAX_BREITE: u32 = 640;

/// Bild der Aufnahme als egui-Bild, höchstens `max_breite` breit. `None`,
/// wenn der Rahmen leer oder kaputt ist.
pub fn bild(rahmen: &SharedFrame, max_breite: u32) -> Option<egui::ColorImage> {
    let (w, h) = (rahmen.w as usize, rahmen.h as usize);
    if w == 0 || h == 0 || rahmen.data.len() < w * h * 3 {
        return None;
    }
    let max = max_breite.max(1) as usize;
    let (zw, zh) = if w > max {
        (max, (h * max / w).max(1))
    } else {
        (w, h)
    };
    let mut rgb = Vec::with_capacity(zw * zh * 3);
    for y in 0..zh {
        let qy = y * h / zh;
        for x in 0..zw {
            let qx = x * w / zw;
            let i = (qy * w + qx) * 3;
            rgb.extend_from_slice(&rahmen.data[i..i + 3]);
        }
    }
    Some(egui::ColorImage::from_rgb([zw, zh], &rgb))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn rahmen(w: u32, h: u32, daten: Vec<u8>) -> SharedFrame {
        SharedFrame {
            w,
            h,
            data: Arc::new(daten),
            seq: 1,
        }
    }

    #[test]
    fn kleines_bild_bleibt_wie_es_ist() {
        let b = bild(&rahmen(2, 1, vec![255, 0, 0, 0, 0, 255]), 640).unwrap();
        assert_eq!(b.size, [2, 1]);
        assert_eq!(b.pixels[0], egui::Color32::from_rgb(255, 0, 0));
        assert_eq!(b.pixels[1], egui::Color32::from_rgb(0, 0, 255));
    }

    #[test]
    fn grosses_bild_wird_im_seitenverhaeltnis_kleiner() {
        let b = bild(&rahmen(1920, 1080, vec![7; 1920 * 1080 * 3]), 640).unwrap();
        assert_eq!(b.size, [640, 360]);
    }

    #[test]
    fn kaputter_rahmen_gibt_kein_bild() {
        assert!(bild(&rahmen(4, 4, vec![0; 10]), 640).is_none());
        assert!(bild(&rahmen(0, 4, vec![]), 640).is_none());
    }
}
