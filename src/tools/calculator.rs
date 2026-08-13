use schemars::JsonSchema;
use serde::Deserialize;

pub fn calculation(operator: &str, first_number:f64, second_number:f64) -> anyhow::Result<f64> {
    match operator {
        "add" => Ok(first_number + second_number),
        "subtract" => Ok(first_number - second_number),
        "multiply" => Ok(first_number * second_number),
        "divide" => {
            if second_number == 0.0 {
                Err(anyhow::Error::msg("Cannot divide by zero"))
            }else {
                Ok(first_number / second_number)
            }
        },
        _ => Err(anyhow::Error::msg("Unknown operator"))
    }
}

#[derive(Debug,Deserialize,JsonSchema)]
pub struct CalculatorArgs {
    pub operator: String,
    pub first_number: f64,
    pub second_number: f64
}

#[cfg(test)]
mod tests {
    use super::calculation;

    #[test]
    fn divide_by_zero_returns_error() {
        assert!(calculation("divide", 10.0, 0.0).is_err());
    }

    #[test]
    fn zero_divided_by_number_is_ok() {
        assert_eq!(calculation("divide", 0.0, 5.0).unwrap(), 0.0);
    }

    #[test]
    fn basic_operations() {
        assert_eq!(calculation("add", 1.0, 2.0).unwrap(), 3.0);
        assert_eq!(calculation("subtract", 5.0, 2.0).unwrap(), 3.0);
        assert_eq!(calculation("multiply", 3.0, 4.0).unwrap(), 12.0);
        assert_eq!(calculation("divide", 8.0, 2.0).unwrap(), 4.0);
    }

    #[test]
    fn unknown_operator_returns_error() {
        assert!(calculation("mod", 1.0, 2.0).is_err());
    }
}
