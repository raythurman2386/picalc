//! The calculator engine, ported from omacalc's `backend.cpp`.
//!
//! Pure Rust with no GPUI imports so the whole interaction model is
//! unit-testable. The UI layer drives exactly one entry point —
//! [`Calculator::press`] — for keypad clicks and keyboard bindings alike.

/// A key press as dispatched by the keypad or the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Digit(u8),
    Decimal,
    Add,
    Subtract,
    Multiply,
    Divide,
    Percent,
    Equals,
    Sign,
    Backspace,
    Clear,
}

/// The pretty operator glyphs the expression line is spelled with.
pub const PLUS: &str = "+";
pub const MINUS: &str = "\u{2212}"; // −
pub const MULTIPLY: &str = "\u{d7}"; // ×
pub const DIVIDE: &str = "\u{f7}"; // ÷

fn is_operator(token: &str) -> bool {
    token == PLUS || token == MINUS || token == MULTIPLY || token == DIVIDE
}

/// Seal a raw entry into a plain number token. Digits are entered raw, so
/// "5." and "-" can linger while typing; joining them into the expression
/// unsealed would produce unparseable tokens.
fn seal_number(entry: &str) -> String {
    let sealed = entry.strip_suffix('.').unwrap_or(entry);
    if sealed.is_empty() || sealed == "-" {
        "0".to_string()
    } else {
        sealed.to_string()
    }
}

/// The key both the keypad and the keyboard dispatch through, mirroring
/// omacalc's `pressKey` mapping. Returns None for keys this calculator
/// does not handle.
pub fn key_from_str(key: &str) -> Option<Key> {
    if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
        return Some(Key::Digit(key.as_bytes()[0] - b'0'));
    }
    match key {
        "." | "," => Some(Key::Decimal),
        "+" => Some(Key::Add),
        "-" | MINUS => Some(Key::Subtract),
        "*" | MULTIPLY => Some(Key::Multiply),
        "/" | DIVIDE => Some(Key::Divide),
        "%" => Some(Key::Percent),
        "=" => Some(Key::Equals),
        "sign" => Some(Key::Sign),
        "backspace" => Some(Key::Backspace),
        "clear" => Some(Key::Clear),
        _ => None,
    }
}

/// The calculator state: the expression tokens, the entry being typed, and
/// the just-evaluated / errored flags that steer what the next key does.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Calculator {
    tokens: Vec<String>,
    entry: String,
    result: String,
    result_value: f64,
    evaluated_expression: String,
    just_evaluated: bool,
    errored: bool,
}

impl Calculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// The expression line shown above the result. After `=` it keeps the
    /// evaluated expression visible; while typing it shows the tokens.
    pub fn expression(&self) -> String {
        if self.errored || self.just_evaluated {
            return self.evaluated_expression.clone();
        }
        pretty_expression(&self.tokens)
    }

    /// The result line.
    pub fn display(&self) -> String {
        if self.errored {
            return "Error".to_string();
        }
        if !self.entry.is_empty() {
            return self.entry.clone();
        }
        if self.just_evaluated {
            return self.result.clone();
        }
        format_number(self.current_value().parse().unwrap_or(0.0))
    }

    pub fn is_errored(&self) -> bool {
        self.errored
    }

    /// The single input entry point. Keypad clicks and keyboard bindings
    /// both route here.
    pub fn press(&mut self, key: Key) {
        match key {
            Key::Digit(digit) => self.press_digit(digit),
            Key::Decimal => self.press_decimal(),
            Key::Add => self.press_operator(PLUS),
            Key::Subtract => self.press_operator(MINUS),
            Key::Multiply => self.press_operator(MULTIPLY),
            Key::Divide => self.press_operator(DIVIDE),
            Key::Equals => self.press_equals(),
            Key::Percent => self.press_percent(),
            Key::Sign => self.press_toggle_sign(),
            Key::Backspace => self.press_backspace(),
            Key::Clear => self.press_clear(),
        }
    }

    /// Press a key by its string name (`"7"`, `"+"`, `"sign"`, ...).
    pub fn press_str(&mut self, key: &str) {
        if let Some(key) = key_from_str(key) {
            self.press(key);
        }
    }

    fn press_digit(&mut self, digit: u8) {
        if self.errored {
            self.press_clear();
        }
        if self.just_evaluated {
            // A digit after equals starts a new calculation rather than
            // appending to the result.
            self.press_clear();
        }
        let digit = char::from(b'0' + digit).to_string();

        if self.entry == "0" {
            self.entry = digit;
            return;
        }
        if self.entry == "-0" {
            self.entry = format!("-{digit}");
            return;
        }

        // Fifteen significant digits is what the display format preserves;
        // letting the entry grow beyond that would silently round what was
        // typed. The zero in a leading "0." is not significant, so it does
        // not count.
        let mut digits = self.entry.chars().filter(|c| c.is_ascii_digit()).count();
        if self.entry.starts_with("0.") || self.entry.starts_with("-0.") {
            digits -= 1;
        }
        if digits < 15 {
            self.entry.push_str(&digit);
        }
    }

    fn press_decimal(&mut self) {
        if self.errored {
            self.press_clear();
        }
        if self.just_evaluated {
            self.press_clear();
        }
        if self.entry.is_empty() {
            self.entry = "0.".to_string();
        } else if !self.entry.contains('.') {
            self.entry.push('.');
        }
    }

    fn press_operator(&mut self, pretty: &str) {
        if self.errored {
            return;
        }

        if self.just_evaluated {
            // Chain from the exact result value, not its rounded display
            // text, so 1 ÷ 3 = × 3 comes back as 1. Seventeen significant
            // digits round-trip any double; the expression line re-rounds
            // them for presentation.
            self.tokens = vec![round_trip(self.result_value)];
            self.clear_evaluation();
        }

        if !self.entry.is_empty() {
            self.tokens.push(seal_number(&self.entry));
            self.tokens.push(pretty.to_string());
            self.entry.clear();
        } else if self.tokens.is_empty() {
            self.tokens.push("0".to_string());
            self.tokens.push(pretty.to_string());
        } else if self
            .tokens
            .last()
            .map(|token| is_operator(token))
            .unwrap_or(false)
        {
            let last = self.tokens.last_mut().expect("checked operator");
            *last = pretty.to_string();
        } else {
            self.tokens.push(pretty.to_string());
        }
    }

    fn press_equals(&mut self) {
        if self.errored || self.just_evaluated {
            return;
        }

        let mut final_tokens = self.tokens.clone();
        if !self.entry.is_empty() {
            final_tokens.push(seal_number(&self.entry));
        }
        while final_tokens
            .last()
            .map(|token| is_operator(token))
            .unwrap_or(false)
        {
            final_tokens.pop();
        }
        if final_tokens.is_empty() {
            return;
        }

        self.evaluated_expression = pretty_expression(&final_tokens);
        match evaluate_tokens(&final_tokens) {
            Some(value) => {
                self.result_value = value;
                self.result = format_number(value);
                self.just_evaluated = true;
            }
            None => {
                self.errored = true;
            }
        }
        self.tokens.clear();
        self.entry.clear();
    }

    /// iOS-style percent: with a pending + or −, x% means x percent of the
    /// running total, so 200 + 10 % = gives 220. With × or ÷ (or on its
    /// own) x% is simply x ÷ 100, so 200 × 10 % = gives 20.
    fn press_percent(&mut self) {
        if self.errored {
            return;
        }

        if self.just_evaluated {
            let percent = self.result_value / 100.0;
            self.clear_evaluation();
            self.entry = format_number(percent);
            return;
        }

        let Ok(value) = self.current_value().parse::<f64>() else {
            return;
        };

        let mut percent = value / 100.0;
        if let Some(last) = self.tokens.last() {
            if last == PLUS || last == MINUS {
                let mut left_side = self.tokens.clone();
                left_side.pop();
                if let Some(base) = evaluate_tokens(&left_side) {
                    percent = base * value / 100.0;
                }
            }
        }
        self.entry = format_number(percent);
    }

    fn press_toggle_sign(&mut self) {
        if self.errored {
            return;
        }
        if self.just_evaluated {
            self.begin_editing_after_result();
        }

        // With nothing typed yet, start a fresh negative operand instead of
        // dredging up the previous one: 4 + ± 2 enters -2, not -42.
        if self.entry.is_empty() {
            self.entry = "-0".to_string();
            return;
        }

        if let Some(stripped) = self.entry.strip_prefix('-') {
            self.entry = stripped.to_string();
        } else {
            self.entry.insert(0, '-');
        }
    }

    fn press_backspace(&mut self) {
        if self.errored {
            self.press_clear();
            return;
        }
        if self.just_evaluated {
            self.begin_editing_after_result();
        }

        self.entry.pop();
        if self.entry == "-" {
            self.entry.clear();
        }
    }

    fn press_clear(&mut self) {
        self.tokens.clear();
        self.entry.clear();
        self.clear_evaluation();
        self.errored = false;
    }

    /// Editing after equals picks up from the result's displayed digits,
    /// with the old expression cleared away so the new one grows from
    /// "42" rather than "42 × 3 + 7".
    fn begin_editing_after_result(&mut self) {
        self.entry = self.result.clone();
        self.tokens.clear();
        self.clear_evaluation();
    }

    fn clear_evaluation(&mut self) {
        self.result.clear();
        self.result_value = 0.0;
        self.evaluated_expression.clear();
        self.just_evaluated = false;
    }

    /// The number the calculator is "at" right now: the entry being typed,
    /// or the operand the last operator was applied to, or the fresh-start
    /// zero.
    fn current_value(&self) -> String {
        if !self.entry.is_empty() {
            return seal_number(&self.entry);
        }
        for token in self.tokens.iter().rev() {
            if !is_operator(token) {
                return token.clone();
            }
        }
        "0".to_string()
    }

    /// Copy payload: whatever the result line currently shows.
    pub fn copy_text(&self) -> String {
        self.display()
    }

    /// Paste: replace the current entry with the clipboard number, if the
    /// clipboard holds one. Parsing is pure logic; clipboard I/O stays in
    /// the UI layer.
    pub fn paste_text(&mut self, text: &str) {
        let Some(value) = parse_pasted_number(text) else {
            return;
        };
        if self.errored || self.just_evaluated {
            self.press_clear();
        }
        self.entry = format_number(value);
    }
}

/// Operand tokens carry full round-trip precision when they come from a
/// chained result; present every number at display precision instead.
pub fn pretty_expression(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| {
            if is_operator(token) {
                token.clone()
            } else {
                format_number(token.parse().unwrap_or(0.0))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fold × and ÷ into their neighbors first, then sum what remains, giving
/// multiplication its usual precedence over addition. Returns None when the
/// token list is malformed or the result is not finite (divide by zero).
pub fn evaluate_tokens(tokens: &[String]) -> Option<f64> {
    if tokens.len().is_multiple_of(2) {
        return None;
    }

    let mut values: Vec<f64> = vec![tokens[0].parse::<f64>().ok()?];
    let mut additive_operators: Vec<&str> = Vec::new();

    for pair in tokens[1..].chunks_exact(2) {
        let op = pair[0].as_str();
        let operand = pair[1].parse::<f64>().ok()?;
        if !is_operator(op) {
            return None;
        }
        match op {
            MULTIPLY => *values.last_mut()? *= operand,
            DIVIDE => *values.last_mut()? /= operand,
            _ => {
                additive_operators.push(op);
                values.push(operand);
            }
        }
    }

    let mut total = values[0];
    for (i, op) in additive_operators.iter().enumerate() {
        let operand = *values.get(i + 1)?;
        if *op == PLUS {
            total += operand;
        } else {
            total -= operand;
        }
    }

    if total.is_finite() {
        Some(total)
    } else {
        None
    }
}

/// Format a number for the display like Qt's `QString::number(value, 'g',
/// 15)`: fifteen significant digits keeps binary-float noise like
/// 0.1 + 0.2 = 0.30000000000000004 out of the display while showing every
/// integer the 15-digit entry limit can produce exactly; larger magnitudes
/// fall back to scientific notation. Trailing zeros are stripped and
/// negative zero collapses to zero.
pub fn format_number(value: f64) -> String {
    let value = if value == 0.0 { 0.0 } else { value };
    if value == 0.0 {
        return "0".to_string();
    }

    // The exponent decides the style, exactly as C's %g: scientific when
    // exp < -4 or exp >= 15, fixed otherwise.
    let shortest = format!("{value:e}");
    let exp: i32 = shortest[shortest.rfind('e').expect("scientific form") + 1..]
        .parse()
        .expect("exponent digits");

    if !(-4..15).contains(&exp) {
        let scientific = format!("{value:.14e}");
        let split = scientific.rfind('e').expect("scientific form");
        let (mantissa, exponent) = scientific.split_at(split);
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        let digits = &exponent[1..];
        let (sign, digits) = match digits.strip_prefix('-') {
            Some(digits) => ("-", digits),
            None => ("+", digits),
        };
        // C and Qt pad the exponent to two digits: 1e-05, not 1e-5.
        if digits.len() < 2 {
            format!("{mantissa}e{sign}0{digits}")
        } else {
            format!("{mantissa}e{sign}{digits}")
        }
    } else {
        let precision = (14 - exp).max(0) as usize;
        let fixed = format!("{value:.precision$}");
        if fixed.contains('.') {
            fixed
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            fixed
        }
    }
}

/// A value formatted so `parse` recovers it exactly: seventeen significant
/// digits round-trip any double.
fn round_trip(value: f64) -> String {
    format!("{value:.16e}")
}

/// Parse a pasted number, tolerating surrounding whitespace, a decimal
/// comma, and the typographic minus this calculator itself puts in
/// expressions.
pub fn parse_pasted_number(text: &str) -> Option<f64> {
    let text = text.trim().replace(MINUS, "-").replace(' ', "");
    if let Ok(value) = text.parse::<f64>() {
        if value.is_finite() {
            return Some(value);
        }
        return None;
    }
    // A decimal comma is welcome; anything else unparseable is ignored.
    let text = text.replace(',', ".");
    if let Ok(value) = text.parse::<f64>() {
        if value.is_finite() {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc(keys: &str) -> Calculator {
        let mut calculator = Calculator::new();
        for key in keys.split(' ').filter(|k| !k.is_empty()) {
            calculator.press_str(key);
        }
        calculator
    }

    fn run(keys: &str) -> String {
        calc(keys).display()
    }

    #[test]
    fn starts_at_zero() {
        let calculator = Calculator::new();
        assert_eq!(calculator.display(), "0");
        assert_eq!(calculator.expression(), "");
    }

    #[test]
    fn calculates_with_precedence() {
        let calculator = calc("4 2 × 3 + 7 =");
        assert_eq!(calculator.display(), "133");
        assert_eq!(calculator.expression(), "42 × 3 + 7");

        assert_eq!(run("clear 2 + 3 × 4 ="), "14");
        assert_eq!(run("clear 1 0 - 4 ÷ 2 ="), "8");
    }

    #[test]
    fn expression_reads_back_as_entered() {
        // After = the expression stays visible above the result and reads
        // back exactly as entered, with the pretty operator glyphs.
        let calculator = calc("4 2 × 3 + 7 =");
        assert_eq!(calculator.display(), "133");
        assert_eq!(calculator.expression(), "42 × 3 + 7");
        assert_eq!(calculator.expression(), "42 \u{d7} 3 + 7");

        let calculator = calc("1 2 - 5 =");
        assert_eq!(calculator.expression(), "12 \u{2212} 5");

        let calculator = calc("8 ÷ 2 =");
        assert_eq!(calculator.expression(), "8 \u{f7} 2");
    }

    #[test]
    fn shows_entry_while_typing() {
        let calculator = calc("4 2 ×");
        assert_eq!(calculator.display(), "42");
        assert_eq!(calculator.expression(), "42 ×");

        let calculator = calc("4 2 × 3");
        assert_eq!(calculator.display(), "3");
    }

    #[test]
    fn handles_decimals() {
        assert_eq!(run(". 5 + . 2 5 ="), "0.75");

        // Binary-float noise stays out of the display.
        assert_eq!(run("clear 0 . 1 + 0 . 2 ="), "0.3");

        // A second decimal point in one number is ignored.
        assert_eq!(run("clear 1 . 5 . 5"), "1.55");
    }

    #[test]
    fn division_by_zero_errors() {
        let calculator = calc("1 ÷ 0 =");
        assert_eq!(calculator.display(), "Error");

        // Digits recover from an error without an explicit clear.
        let mut calculator = calculator;
        calculator.press_str("5");
        assert_eq!(calculator.display(), "5");
    }

    #[test]
    fn divide_by_zero_then_digit_starts_fresh() {
        let calculator = calc("5 ÷ 0 = 5");
        assert_eq!(calculator.display(), "5");
        assert_eq!(calculator.expression(), "");
    }

    #[test]
    fn percent_of_running_total() {
        // With a pending + or −, x% means x percent of the running total.
        assert_eq!(run("2 0 0 + 1 0 % ="), "220");
        assert_eq!(run("clear 2 0 0 - 1 0 % ="), "180");

        // With × or ÷, or standalone, x% is simply x ÷ 100.
        assert_eq!(run("clear 2 0 0 × 1 0 % ="), "20");
        assert_eq!(run("clear 5 0 %"), "0.5");

        // After equals, percent picks up from the result.
        assert_eq!(run("clear 4 0 + 1 0 = %"), "0.5");
    }

    #[test]
    fn percent_and_sign() {
        let mut calculator = calc("8 sign");
        assert_eq!(calculator.display(), "-8");
        calculator.press_str("sign");
        assert_eq!(calculator.display(), "8");

        assert_eq!(run("clear 4 + 8 sign ="), "-4");
    }

    #[test]
    fn sign_starts_new_operand() {
        // Sign with nothing typed starts a fresh negative operand rather
        // than negating the previous one: 4 + ± 2 = is 4 + (-2), not 4 + (-42).
        let calculator = calc("4 + sign 2 =");
        assert_eq!(calculator.display(), "2");
        assert_eq!(calculator.expression(), "4 + -2");

        let calculator = calc("clear sign");
        assert_eq!(calculator.display(), "-0");
        let mut calculator = calculator;
        calculator.press_str("5");
        assert_eq!(calculator.display(), "-5");
    }

    #[test]
    fn chains_with_full_precision() {
        // Chaining continues from the exact value, not the rounded display.
        assert_eq!(run("1 ÷ 3 = × 3 ="), "1");

        // Integers within the 15-digit entry limit survive exactly.
        assert_eq!(run("clear 9 9 9 9 9 9 9 9 9 9 9 9 9 9 ="), "99999999999999");
    }

    #[test]
    fn caps_entry_at_fifteen_digits() {
        assert_eq!(run("1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7 8"), "123456789123456");

        // The decimal point does not count against the digit cap.
        assert_eq!(
            run("clear . 1 2 3 4 5 6 7 8 9 1 2 3 4 5 6 7"),
            "0.123456789123456"
        );
    }

    #[test]
    fn backspace_edits() {
        let calculator = calc("1 2 3 backspace");
        assert_eq!(calculator.display(), "12");

        let calculator = calc("1 2 3 backspace backspace backspace");
        assert_eq!(calculator.display(), "0");
    }

    #[test]
    fn backspace_after_result_edits_result_digits() {
        let calculator = calc("4 2 = backspace");
        assert_eq!(calculator.display(), "4");
    }

    #[test]
    fn chains_from_result() {
        let calculator = calc("6 × 7 = × 2 =");
        assert_eq!(calculator.display(), "84");
        assert_eq!(calculator.expression(), "42 × 2");

        // A digit after equals starts fresh instead of appending to the result.
        let mut calculator = calculator;
        calculator.press_str("9");
        assert_eq!(calculator.display(), "9");
        assert_eq!(calculator.expression(), "");
    }

    #[test]
    fn replaces_dangling_operator() {
        assert_eq!(run("4 + × 2 ="), "8");

        // Equals with a trailing operator drops it.
        assert_eq!(run("clear 9 + ="), "9");
    }

    #[test]
    fn pastes_numbers() {
        let mut calculator = Calculator::new();

        calculator.paste_text(" 42.5 ");
        assert_eq!(calculator.display(), "42.5");

        // A pasted number is a normal entry that calculates like any other;
        // the decimal point after the operator starts a fresh "0.5".
        for key in ["+", ".", "5", "="] {
            calculator.press_str(key);
        }
        assert_eq!(calculator.display(), "43");

        // Decimal commas are welcome; garbage is ignored.
        calculator.paste_text("1,5");
        assert_eq!(calculator.display(), "1.5");

        calculator.paste_text("not a number");
        assert_eq!(calculator.display(), "1.5");
    }

    #[test]
    fn paste_tolerates_typographic_minus_and_comma() {
        let mut calculator = Calculator::new();
        calculator.paste_text("\u{2212}7,25 ");
        assert_eq!(calculator.display(), "-7.25");
    }

    #[test]
    fn paste_after_result_replaces() {
        let mut calculator = calc("6 × 7 =");
        calculator.paste_text("12");
        assert_eq!(calculator.display(), "12");
        assert_eq!(calculator.expression(), "");
    }

    #[test]
    fn evaluates_tokens() {
        let tokens = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            evaluate_tokens(&tokens(&["2", "+", "3", "\u{d7}", "4"])),
            Some(14.0)
        );
        assert_eq!(evaluate_tokens(&tokens(&["2", "+"])), None);
        assert_eq!(evaluate_tokens(&tokens(&["1", "\u{f7}", "0"])), None);
    }

    #[test]
    fn formats_numbers() {
        assert_eq!(format_number(133.0), "133");
        assert_eq!(format_number(0.1 + 0.2), "0.3");
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(1e15), "1e+15");
        assert_eq!(format_number(1e-5), "1e-05");
        assert_eq!(format_number(123456.789), "123456.789");
        assert_eq!(format_number(1234567.0), "1234567");
        assert_eq!(format_number(0.0001), "0.0001");
        assert_eq!(format_number(0.5), "0.5");
        assert_eq!(format_number(99999999999999.0), "99999999999999");
    }

    #[test]
    fn key_mapping_covers_keyboard_and_keypad() {
        assert_eq!(key_from_str("7"), Some(Key::Digit(7)));
        assert_eq!(key_from_str("0"), Some(Key::Digit(0)));
        assert_eq!(key_from_str("+"), Some(Key::Add));
        assert_eq!(key_from_str("-"), Some(Key::Subtract));
        assert_eq!(key_from_str(MINUS), Some(Key::Subtract));
        assert_eq!(key_from_str("*"), Some(Key::Multiply));
        assert_eq!(key_from_str(MULTIPLY), Some(Key::Multiply));
        assert_eq!(key_from_str("/"), Some(Key::Divide));
        assert_eq!(key_from_str(DIVIDE), Some(Key::Divide));
        assert_eq!(key_from_str("%"), Some(Key::Percent));
        assert_eq!(key_from_str("="), Some(Key::Equals));
        assert_eq!(key_from_str("."), Some(Key::Decimal));
        assert_eq!(key_from_str(","), Some(Key::Decimal));
        assert_eq!(key_from_str("sign"), Some(Key::Sign));
        assert_eq!(key_from_str("backspace"), Some(Key::Backspace));
        assert_eq!(key_from_str("clear"), Some(Key::Clear));
        assert_eq!(key_from_str("x"), None);
    }

    #[test]
    fn copy_returns_display() {
        let calculator = calc("4 2");
        assert_eq!(calculator.copy_text(), "42");
        let calculator = calc("4 2 × 3 + 7 =");
        assert_eq!(calculator.copy_text(), "133");
    }
}
