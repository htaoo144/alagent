use schemars::JsonSchema;
use serde::Deserialize;

pub fn calculation(operator: &str, first_number:f64, second_number:f64) -> anyhow::Result<f64> {
    match operator {
        "add" => Ok(first_number + second_number),
        "subtract" => Ok(first_number - second_number),
        "multiply" => Ok(first_number * second_number),
        "divide" => {
            if first_number == 0.0 {
                Err(anyhow::Error::msg("Cannot divide by zero"))
            }else {
                Ok((first_number / second_number) as f64)
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