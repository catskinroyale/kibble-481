//! Where each photo sits, in whole frames, and how it fades (D33): one schedule that the
//! exported timeline (`resolve::fcpxml`) and the video (`video`) both follow, so the two
//! agree to the frame.
//!
//! A shot sits on `from..to`. A fade in of `fade_in` frames is centred on `from`: from the
//! shot before (a cross dissolve) when that ends at `from`, else from black. A fade out of
//! `fade_out` frames is centred on `to`, to black. Every fade is an even number of frames,
//! so it centres on a frame.

use crate::slideshow::{Plan, Slideshow, Transition};

/// One photo on the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shot {
    /// Which photo (and image of the plan).
    pub photo: usize,
    pub from: u64,
    pub to: u64,
    pub fade_in: u64,
    pub fade_out: u64,
}

/// Seconds to whole frames.
pub fn frame(secs: f64, fps: u32) -> u64 {
    (secs.max(0.0) * fps as f64).round() as u64
}

fn even(frames: u64) -> u64 {
    frames / 2 * 2
}

/// The shots of the plan's images, no more than there are photos. `end` is the frame the
/// soundtrack (and the video) ends on, for a final fade stretched to it.
pub fn shots(plan: &Plan, photos: usize, s: &Slideshow, fps: u32, end: u64) -> Vec<Shot> {
    let at = |secs: f64| frame(secs, fps);
    let spans: Vec<(u64, u64)> = plan.spans().take(photos).map(|(a, b)| (at(a), at(b).max(at(a) + 1))).collect();
    let n = spans.len();
    let shortest = spans.iter().map(|(a, b)| b - a).min().unwrap_or(0);
    let plain = |i: usize, (a, b): (u64, u64)| Shot { photo: i, from: a, to: b, fade_in: 0, fade_out: 0 };
    match s.transition {
        Transition::Cut => spans.into_iter().enumerate().map(|(i, s)| plain(i, s)).collect(),
        Transition::Crossfade | Transition::FadeToBlack => {
            // centred on each change, at most half the shortest image
            let d = even(at(s.fade as f64).min(shortest / 2));
            let dip = s.transition == Transition::FadeToBlack;
            spans
                .into_iter()
                .enumerate()
                .map(|(i, (a, b))| {
                    let last = i + 1 == n;
                    Shot {
                        photo: i,
                        from: a,
                        // to black: each but the last gives its final `d` frames to black
                        to: if dip && !last { (b - d).max(a + 1) } else { b },
                        fade_in: if i > 0 { d } else { 0 },
                        fade_out: if dip && !last { d } else { 0 },
                    }
                })
                .collect()
        }
        Transition::FadeIn | Transition::FadeOut | Transition::FadeInOut => {
            let (fi, fo) = s.edge_fades();
            let (mut fi, mut fo) = (at(fi), at(fo));
            // both within the shortest image
            if fi + fo > shortest {
                let k = shortest as f64 / (fi + fo) as f64;
                (fi, fo) = ((fi as f64 * k).floor() as u64, (fo as f64 * k).floor() as u64);
            }
            let (fi, fo) = (even(fi), even(fo));
            let mut out: Vec<Shot> = spans
                .iter()
                .enumerate()
                .map(|(i, &(a, b))| Shot { photo: i, from: a + fi / 2, to: b - fo / 2, fade_in: fi, fade_out: fo })
                .collect();
            // the first image's own fade up from black, within its time: fading out only, it
            // may still fade in; fading in and out, it may take longer (or shorter) than the rest
            if (s.transition == Transition::FadeOut || s.first_fade_in > 0.0)
                && let (Some(first), Some(&(a, b))) = (out.first_mut(), spans.first())
            {
                // its own fade in first, in full (keeping at least a quarter of the image,
                // or the others' fade out if shorter, to fade out in); its fade out in what's left
                let len = b - a;
                let up = even(at(s.first_fade_in.max(0.0) as f64).min(len - fo.min(len / 4)));
                let down = even(fo.min(len - up));
                (first.from, first.fade_in, first.to, first.fade_out) = (a + up / 2, up, b - down / 2, down);
            }
            // the last image's fade out drawn out to the end of the soundtrack, so the
            // picture goes to black as the reverb tail dies away
            if s.stretch_final_fade
                && s.transition.fades_out()
                && let (Some(last), Some(&(_, b))) = (out.last_mut(), spans.last())
            {
                // from where its fade would start, an even number of frames to the end
                let start = (b - last.fade_out).max(last.from);
                if end > start + 1 {
                    let len = even(end - start);
                    (last.to, last.fade_out) = (end - len / 2, len);
                }
            }
            out
        }
    }
}

/// How much of each shot is seen at frame `f`, in order (later over earlier, over
/// black): `(shot, opacity 0..=1)` for those seen at all.
pub fn layers(shots: &[Shot], f: u64) -> Vec<(usize, f32)> {
    let mut out = Vec::new();
    for (j, s) in shots.iter().enumerate() {
        let start = s.from - s.fade_in / 2;
        // under the next shot's dissolve, if it starts where this ends
        let under = shots.get(j + 1).filter(|n| n.from == s.to && n.fade_in > 0).map(|n| n.fade_in / 2);
        let end = match under {
            Some(h) => s.to + h,
            None => s.to + s.fade_out / 2,
        };
        if f < start || f >= end {
            continue;
        }
        let mut a = 1.0f32;
        if s.fade_in > 0 && f < s.from + s.fade_in / 2 {
            a = a.min((f - start) as f32 / s.fade_in as f32);
        }
        if under.is_none() && s.fade_out > 0 && f + s.fade_out / 2 > s.to {
            a = a.min((end - f) as f32 / s.fade_out as f32);
        }
        if a > 0.0 {
            out.push((j, a));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slideshow::{Grid, Plan, Slideshow};

    fn plan(s: &Slideshow) -> Plan {
        // 60 bpm in 4/4: 4 s bars, 8 s images after a two-bar intro, from frame 200 at 25 fps
        Plan::for_track(s, Grid::new(60.0, (4, 4)), 60.0)
    }

    fn alpha(shots: &[Shot], f: u64, j: usize) -> f32 {
        layers(shots, f).into_iter().find(|l| l.0 == j).map_or(0.0, |l| l.1)
    }

    #[test]
    fn fade_in_rises_from_black_at_each_start() {
        let s = Slideshow { hold: 8.0, by_chords: false, transition: Transition::FadeIn, fade_in: 2.0, ..Slideshow::default() };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert_eq!(sh[0], Shot { photo: 0, from: 225, to: 400, fade_in: 50, fade_out: 0 });
        // black until the image's start, then up over 2 s (50 frames)
        assert!(layers(&sh, 199).is_empty());
        assert_eq!(alpha(&sh, 200, 0), 0.0);
        assert_eq!(alpha(&sh, 225, 0), 0.5);
        assert_eq!(alpha(&sh, 250, 0), 1.0);
        // a cut at 16 s from the whole first image to black, the second rising
        assert_eq!(alpha(&sh, 399, 0), 1.0);
        assert!(layers(&sh, 400).is_empty());
        assert_eq!(alpha(&sh, 410, 1), 0.2);
    }

    #[test]
    fn fade_out_falls_to_black_by_each_end() {
        let s = Slideshow { hold: 8.0, by_chords: false, transition: Transition::FadeOut, fade_out: 1.0, ..Slideshow::default() };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert_eq!(sh[0], Shot { photo: 0, from: 200, to: 388, fade_in: 0, fade_out: 24 });
        assert_eq!(alpha(&sh, 200, 0), 1.0);
        assert_eq!(alpha(&sh, 376, 0), 1.0);
        assert_eq!(alpha(&sh, 388, 0), 0.5);
        // black on the last frame before the change, the next image whole on it
        assert!(alpha(&sh, 399, 0) < 0.05);
        assert_eq!(layers(&sh, 400), vec![(1, 1.0f32)]);
    }

    #[test]
    fn in_and_out_each_their_own_length_within_the_image() {
        let s = Slideshow {
            hold: 8.0,
            by_chords: false,
            transition: Transition::FadeInOut,
            fade_in: 3.0,
            fade_out: 1.0,
            ..Slideshow::default()
        };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert_eq!(sh[0], Shot { photo: 0, from: 237, to: 388, fade_in: 74, fade_out: 24 });
        assert_eq!(alpha(&sh, 200, 0), 0.0);
        assert_eq!(alpha(&sh, 274, 0), 1.0);
        assert_eq!(alpha(&sh, 300, 0), 1.0);
        // longer than the image: both shrink to fit it
        let s = Slideshow { fade_in: 6.0, fade_out: 6.0, ..s };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert!(sh.iter().all(|x| x.fade_in + x.fade_out <= 200 && x.to > x.from));
        // never two images at once
        assert!((0..1500).all(|f| layers(&sh, f).len() <= 1));
    }

    #[test]
    fn fading_out_the_first_may_fade_in_and_the_last_fade_to_the_end() {
        let s = Slideshow {
            hold: 8.0,
            by_chords: false,
            transition: Transition::FadeOut,
            fade_out: 1.0,
            first_fade_in: 2.0,
            stretch_final_fade: true,
            ..Slideshow::default()
        };
        let p = plan(&s);
        // the soundtrack rings on 10 s after the music
        let end = ((p.total + 10.0) * 25.0) as u64;
        let sh = shots(&p, 99, &s, 25, end);
        // the first rises from black over 2 s, then fades out as the others do
        assert_eq!(sh[0], Shot { photo: 0, from: 225, to: 388, fade_in: 50, fade_out: 24 });
        assert_eq!(alpha(&sh, 200, 0), 0.0);
        assert_eq!(alpha(&sh, 250, 0), 1.0);
        // the others cut in
        assert_eq!(sh[1].fade_in, 0);
        // the last fades from where it would have begun to, right to the last frame
        let last = *sh.last().unwrap();
        let b = (p.end * 25.0).round() as u64;
        assert_eq!(last.to + last.fade_out / 2, end);
        assert_eq!(last.to - last.fade_out / 2, b - 24);
        assert_eq!(alpha(&sh, b - 24, sh.len() - 1), 1.0);
        let (mid, tail) = (alpha(&sh, b + 125, sh.len() - 1), alpha(&sh, end - 1, sh.len() - 1));
        assert!(mid > 0.3 && mid < 1.0 && tail > 0.0 && tail < 0.01, "{mid} {tail}");
        assert!(layers(&sh, end).is_empty());
        // without the stretch, black by the end of the last image
        let s = Slideshow { stretch_final_fade: false, first_fade_in: 0.0, ..s };
        let sh = shots(&p, 99, &s, 25, end);
        assert_eq!(sh[0].fade_in, 0);
        assert!(layers(&sh, b).is_empty());
    }

    #[test]
    fn fading_in_and_out_the_first_may_fade_in_on_its_own() {
        let s = Slideshow {
            hold: 8.0,
            by_chords: false,
            transition: Transition::FadeInOut,
            fade_in: 1.0,
            fade_out: 1.0,
            first_fade_in: 4.0,
            ..Slideshow::default()
        };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        // the first over 4 s (100 frames), the rest over 1 s as before
        assert_eq!(sh[0], Shot { photo: 0, from: 250, to: 388, fade_in: 100, fade_out: 24 });
        assert_eq!(sh[1], Shot { photo: 1, from: 412, to: 588, fade_in: 24, fade_out: 24 });
        assert_eq!(alpha(&sh, 250, 0), 0.5);
        // 0: the first as the others
        let s = Slideshow { first_fade_in: 0.0, ..s };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert_eq!(sh[0].fade_in, 24);
        // fades longer than the images (8 s images, 6 s each way): the others share the
        // image, 4 s each; the first rises over its full 5 s, fading out in the 3 s left
        let s = Slideshow { fade_in: 6.0, fade_out: 6.0, first_fade_in: 5.0, ..s };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert_eq!(sh[1], Shot { photo: 1, from: 450, to: 550, fade_in: 100, fade_out: 100 });
        assert_eq!(sh[0], Shot { photo: 0, from: 262, to: 362, fade_in: 124, fade_out: 76 });
        assert!((0..1500).all(|f| layers(&sh, f).len() <= 1));
    }

    #[test]
    fn crossfades_mix_the_two_and_cuts_dont() {
        let s = Slideshow { hold: 8.0, by_chords: false, transition: Transition::Crossfade, fade: 2.0, ..Slideshow::default() };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        // centred on the change at 16 s: the first whole beneath, the second rising
        assert_eq!(layers(&sh, 375), vec![(0, 1.0)]);
        assert_eq!(layers(&sh, 400), vec![(0, 1.0), (1, 0.5)]);
        assert_eq!(layers(&sh, 425), vec![(1, 1.0)]);
        let s = Slideshow { transition: Transition::Cut, ..s };
        let sh = shots(&plan(&s), 99, &s, 25, 99_999);
        assert_eq!(layers(&sh, 399), vec![(0, 1.0)]);
        assert_eq!(layers(&sh, 400), vec![(1, 1.0)]);
    }
}
