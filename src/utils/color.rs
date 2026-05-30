// SPDX-License-Identifier: GPL-3.0

/// Calculates perceived luminance of a color (0.0 = dark, 1.0 = bright)
/// Uses the formula for relative luminance from WCAG 2.0
pub fn luminance(color: cosmic::iced::Color) -> f32 {
    // sRGB to linear conversion
    let to_linear = |c: f32| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = to_linear(color.r);
    let g = to_linear(color.g);
    let b = to_linear(color.b);
    // Weighted sum for perceived brightness
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Returns true if dark text should be used on a given background color
pub fn should_use_dark_text(bg: cosmic::iced::Color) -> bool {
    luminance(bg) > 0.45 // Slightly higher threshold for better contrast
}
