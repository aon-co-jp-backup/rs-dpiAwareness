//! 論理ピクセル(96dpi基準)⇔物理ピクセルの換算。
//!
//! 丸めはWindowsの`MulDiv`と同じ(最も近い整数へ、ちょうど半分は0から遠い方へ)。Windows自身の
//! 換算(`GetSystemMetricsForDpi`など)と1ピクセルずれないようにするため。

/// 基準のDPI(100%表示)。
pub const BASE_DPI: u32 = 96;

/// DPIから拡大率(1.0=100%)。`dpi`が0なら1.0。
pub fn factor(dpi: u32) -> f64 {
    if dpi == 0 {
        1.0
    } else {
        dpi as f64 / BASE_DPI as f64
    }
}

/// DPIから拡大率をパーセントで(96→100、120→125、144→150、192→200)。
pub fn percent(dpi: u32) -> u32 {
    (factor(dpi) * 100.0).round() as u32
}

/// `value * num / den`を最も近い整数へ丸める(`MulDiv`と同じ。`den`が0なら`value`をそのまま返す)。
pub fn mul_div(value: i32, num: i32, den: i32) -> i32 {
    if den == 0 {
        return value;
    }
    let p = value as i64 * num as i64;
    let d = den as i64;
    let negative = (p < 0) != (d < 0);
    let (ap, ad) = (p.abs(), d.abs());
    let q = (ap + ad / 2) / ad;
    let q = if negative { -q } else { q };
    q.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// 論理ピクセル(96dpi基準)を、`dpi`の物理ピクセルへ。例: `px(100, 144) == 150`。
pub fn px(logical: i32, dpi: u32) -> i32 {
    mul_div(logical, dpi as i32, BASE_DPI as i32)
}

/// 物理ピクセルを、論理ピクセル(96dpi基準)へ。例: `unpx(150, 144) == 100`。
pub fn unpx(physical: i32, dpi: u32) -> i32 {
    if dpi == 0 {
        return physical;
    }
    mul_div(physical, BASE_DPI as i32, dpi as i32)
}

/// `from`のDPIでの大きさを`to`のDPIでの大きさへ(モニター間の移動や`WM_DPICHANGED`で使う)。
pub fn rescale(value: i32, from: u32, to: u32) -> i32 {
    if from == 0 {
        return value;
    }
    mul_div(value, to as i32, from as i32)
}

/// 小数のまま換算する(フォントサイズなど)。
pub fn pxf(logical: f64, dpi: u32) -> f64 {
    logical * factor(dpi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_scales() {
        assert_eq!(
            (
                percent(96),
                percent(120),
                percent(144),
                percent(168),
                percent(192),
                percent(288)
            ),
            (100, 125, 150, 175, 200, 300)
        );
        assert_eq!(factor(0), 1.0);
    }

    #[test]
    fn px_matches_the_obvious_cases() {
        assert_eq!(px(100, 96), 100);
        assert_eq!(px(100, 144), 150);
        assert_eq!(px(100, 192), 200);
        assert_eq!(px(0, 144), 0);
        assert_eq!(unpx(150, 144), 100);
        assert_eq!(unpx(200, 192), 100);
    }

    #[test]
    fn rounding_is_half_away_from_zero_like_muldiv() {
        // 1 * 120 / 96 = 1.25 → 1、 2 * 120 / 96 = 2.5 → 3(MulDivは半分を遠い方へ)
        assert_eq!(px(1, 120), 1);
        assert_eq!(px(2, 120), 3);
        assert_eq!(px(-2, 120), -3);
        assert_eq!(px(3, 120), 4); // 3.75
        assert_eq!(mul_div(5, 1, 2), 3);
        assert_eq!(mul_div(-5, 1, 2), -3);
        assert_eq!(mul_div(7, 3, 0), 7);
    }

    #[test]
    fn rescale_between_monitors_round_trips_for_round_numbers() {
        assert_eq!(rescale(150, 144, 192), 200);
        assert_eq!(rescale(200, 192, 96), 100);
        assert_eq!(rescale(100, 0, 144), 100);
        assert_eq!(rescale(rescale(100, 96, 144), 144, 96), 100);
    }

    #[test]
    fn floating_scale() {
        assert!((pxf(9.0, 144) - 13.5).abs() < 1e-9);
    }
}
