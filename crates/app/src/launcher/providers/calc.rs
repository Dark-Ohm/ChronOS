//! Calculator provider (`=`): one arithmetic expression, result as a row,
//! Enter copies the result to the clipboard.
//!
//! The evaluator is a hand-rolled recursive-descent parser over
//! `+ - * / % ^` with parentheses and unary minus. The workspace had neither
//! `meval` nor `evalexpr`, and the spec only needs `2+2 → 4` plus graceful
//! errors (`= 1/0` must not crash the shell). A ~120-line parser is the
//! lightest honest option — no CAS crate pulled in.

use super::{ProviderAction, ProviderResult};

/// Build the calculator's result rows for a raw expression.
pub fn results(expr: &str) -> Vec<ProviderResult> {
    let expr = expr.trim();
    if expr.is_empty() {
        return vec![ProviderResult {
            id: "calc-hint".into(),
            label: "type an expression, e.g. 2+2".into(),
            detail: None,
            glyph: '=',
            action: ProviderAction::None,
        }];
    }
    match eval(expr) {
        Ok(value) => {
            let label = format_number(value);
            vec![ProviderResult {
                id: "calc-result".into(),
                label: label.clone(),
                detail: Some(expr.to_string()),
                glyph: '=',
                action: ProviderAction::Copy(label),
            }]
        }
        Err(err) => vec![ProviderResult {
            id: "calc-error".into(),
            label: err,
            detail: Some(expr.to_string()),
            glyph: '⚠',
            action: ProviderAction::None,
        }],
    }
}

/// Evaluate an arithmetic expression. Returns `Err(String)` on parse errors
/// and division/modulo by zero — never panics on bad input.
pub fn eval(src: &str) -> Result<f64, String> {
    let mut parser = Parser {
        chars: src.chars().collect(),
        pos: 0,
    };
    let value = parser.expr()?;
    parser.skip_ws();
    if parser.pos != parser.chars.len() {
        return Err(format!("unexpected '{}'", parser.chars[parser.pos]));
    }
    Ok(value)
}

/// Integral values print without a decimal point; fractional values are
/// rounded to 10 digits and trailing zeros trimmed.
fn format_number(v: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip_ws();
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        self.skip_ws();
        let c = self.chars.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    /// expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Result<f64, String> {
        let mut value = self.term()?;
        loop {
            match self.peek() {
                Some('+') => {
                    self.bump();
                    value += self.term()?;
                }
                Some('-') => {
                    self.bump();
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// term := factor (('*' | '/' | '%') factor)*
    fn term(&mut self) -> Result<f64, String> {
        let mut value = self.factor()?;
        loop {
            match self.peek() {
                Some('*') => {
                    self.bump();
                    value *= self.factor()?;
                }
                Some('/') => {
                    self.bump();
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err("division by zero".into());
                    }
                    value /= divisor;
                }
                Some('%') => {
                    self.bump();
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err("division by zero".into());
                    }
                    value %= divisor;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    /// factor := ('+' | '-') factor | power
    fn factor(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some('+') => {
                self.bump();
                self.factor()
            }
            Some('-') => {
                self.bump();
                Ok(-self.factor()?)
            }
            _ => self.power(),
        }
    }

    /// power := primary ('^' factor)?  (right-associative)
    fn power(&mut self) -> Result<f64, String> {
        let base = self.primary()?;
        if self.peek() == Some('^') {
            self.bump();
            let exponent = self.factor()?;
            Ok(base.powf(exponent))
        } else {
            Ok(base)
        }
    }

    /// primary := '(' expr ')' | number
    fn primary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some('(') => {
                self.bump();
                let value = self.expr()?;
                if self.bump() != Some(')') {
                    return Err("expected ')'".into());
                }
                Ok(value)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) => Err(format!("unexpected '{c}'")),
            None => Err("unexpected end of expression".into()),
        }
    }

    fn number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        let mut has_dot = false;
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == '.' && !has_dot {
                has_dot = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map_err(|_| format!("invalid number '{text}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        assert_eq!(eval("2+2").unwrap(), 4.0);
    }

    #[test]
    fn precedence() {
        assert_eq!(eval("2*3+4").unwrap(), 10.0);
        assert_eq!(eval("(1+2)*3").unwrap(), 9.0);
    }

    #[test]
    fn right_assoc_power() {
        assert_eq!(eval("2^3^2").unwrap(), 512.0);
    }

    #[test]
    fn unary_minus() {
        assert_eq!(eval("-3*2").unwrap(), -6.0);
        assert_eq!(eval("5--2").unwrap(), 7.0);
    }

    #[test]
    fn division_and_modulo() {
        assert_eq!(eval("10/4").unwrap(), 2.5);
        assert_eq!(eval("7%3").unwrap(), 1.0);
    }

    #[test]
    fn division_by_zero_is_an_error_not_a_panic() {
        assert!(eval("1/0").is_err());
        assert!(eval("1%0").is_err());
    }

    #[test]
    fn malformed_expressions_error() {
        assert!(eval("2+").is_err());
        assert!(eval("(1+2").is_err());
        assert!(eval("abc").is_err());
        assert!(eval("").is_err());
    }

    #[test]
    fn integral_formatting() {
        assert_eq!(format_number(4.0), "4");
        assert_eq!(format_number(-12.0), "-12");
        assert_eq!(format_number(2.5), "2.5");
        assert_eq!(format_number(10.0 / 3.0), "3.3333333333");
    }

    #[test]
    fn result_row_copies_value() {
        let rows = results("2+2");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "4");
        assert_eq!(rows[0].action, ProviderAction::Copy("4".to_string()));
    }

    #[test]
    fn error_row_has_no_action() {
        let rows = results("1/0");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, ProviderAction::None);
    }
}
