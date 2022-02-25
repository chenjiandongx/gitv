use chrono::prelude::*;
use datafusion::logical_plan::create_udaf;
use datafusion::physical_plan::udaf::AggregateUDF;
use datafusion::physical_plan::Accumulator;
use datafusion::scalar::ScalarValue;
use datafusion::{
    arrow::{
        array::{ArrayRef, StringArray, UInt64Array},
        datatypes::DataType,
    },
    error::Result,
    physical_plan::{
        functions::{make_scalar_function, Volatility},
        udf::ScalarUDF,
    },
    prelude::*,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn udf_year() -> ScalarUDF {
    let year = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let array = base
            .iter()
            .map(|x| Some(DateTime::parse_from_rfc2822(x.unwrap()).unwrap().year() as u64))
            .collect::<UInt64Array>();

        Ok(Arc::new(array) as ArrayRef)
    };

    let year = make_scalar_function(year);
    create_udf(
        "year",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt64),
        Volatility::Immutable,
        year,
    )
}

pub fn udf_month() -> ScalarUDF {
    let month = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let array = base
            .iter()
            .map(|x| Some(DateTime::parse_from_rfc2822(x.unwrap()).unwrap().month() as u64))
            .collect::<UInt64Array>();

        Ok(Arc::new(array) as ArrayRef)
    };

    let month = make_scalar_function(month);
    create_udf(
        "month",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt64),
        Volatility::Immutable,
        month,
    )
}

pub fn udf_weekday() -> ScalarUDF {
    let weekday = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let array = base
            .iter()
            .map(|x| {
                Some(
                    DateTime::parse_from_rfc2822(x.unwrap())
                        .unwrap()
                        .weekday()
                        .to_string(),
                )
            })
            .collect::<StringArray>();

        Ok(Arc::new(array) as ArrayRef)
    };

    let weekday = make_scalar_function(weekday);
    create_udf(
        "weekday",
        vec![DataType::Utf8],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        weekday,
    )
}

pub fn udf_hour() -> ScalarUDF {
    let hour = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let array = base
            .iter()
            .map(|x| Some(DateTime::parse_from_rfc2822(x.unwrap()).unwrap().hour() as u64))
            .collect::<UInt64Array>();

        Ok(Arc::new(array) as ArrayRef)
    };

    let hour = make_scalar_function(hour);
    create_udf(
        "hour",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt64),
        Volatility::Immutable,
        hour,
    )
}

pub fn udf_timezone() -> ScalarUDF {
    let timezone = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let array = base
            .iter()
            .map(|x| {
                Some(
                    DateTime::parse_from_rfc2822(x.unwrap())
                        .unwrap()
                        .timezone()
                        .to_string(),
                )
            })
            .collect::<StringArray>();

        Ok(Arc::new(array) as ArrayRef)
    };

    let timezone = make_scalar_function(timezone);
    create_udf(
        "timezone",
        vec![DataType::Utf8],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        timezone,
    )
}

fn udf_date(name: &str, format: &'static str) -> ScalarUDF {
    let date = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let array = base
            .iter()
            .map(|x| {
                Some(
                    DateTime::parse_from_rfc2822(x.unwrap())
                        .unwrap()
                        .format(format)
                        .to_string(),
                )
            })
            .collect::<StringArray>();

        Ok(Arc::new(array) as ArrayRef)
    };

    let date = make_scalar_function(date);
    create_udf(
        name,
        vec![DataType::Utf8],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        date,
    )
}

pub fn udf_date_day() -> ScalarUDF {
    udf_date("date_day", "%Y-%m-%d")
}

pub fn udf_date_month() -> ScalarUDF {
    udf_date("date_month", "%Y-%m")
}

pub fn udf_active_days() -> AggregateUDF {
    create_udaf(
        "active_days",
        DataType::Utf8,
        Arc::new(DataType::UInt32),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveDays::new()))),
        Arc::new(vec![DataType::UInt32]),
    )
}

#[derive(Debug)]
struct ActiveDays {
    days: HashSet<String>,
    n: u32,
}

impl ActiveDays {
    pub fn new() -> Self {
        Self {
            days: HashSet::new(),
            n: 0,
        }
    }
}

impl Accumulator for ActiveDays {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        Ok(vec![ScalarValue::from(self.n)])
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        let value = &values[0];
        if let ScalarValue::Utf8(e) = value {
            e.iter()
                .map(|v| {
                    let s = DateTime::parse_from_rfc2822(v)
                        .unwrap()
                        .format("%Y-%m-%d")
                        .to_string();
                    self.days.insert(s);
                })
                .collect()
        };
        self.n = self.days.len() as u32;
        Ok(())
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        let state = &states[0];
        if let ScalarValue::UInt32(Some(n)) = state {
            self.n += n;
        };
        Ok(())
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        Ok(ScalarValue::from(self.n as u32))
    }
}

pub fn udf_active_longest() -> AggregateUDF {
    create_udaf(
        "active_longest",
        DataType::Utf8,
        Arc::new(DataType::Int64),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveLongest::new()))),
        Arc::new(vec![DataType::Int64]),
    )
}

#[derive(Debug)]
struct ActiveLongest {
    data: Vec<i64>,
    n: i64,
}

impl ActiveLongest {
    pub fn new() -> Self {
        Self { data: vec![], n: 0 }
    }

    fn calc_longest(&self, data: &[i64]) -> i64 {
        if data.len() <= 1 {
            return data.len() as i64;
        }

        let mut count: i64 = 1;
        let mut max: i64 = 0;
        for i in 0..data.len() - 1 {
            let k = data[i + 1] - data[i];
            match k {
                0 | 1 => count += k,
                _ => {
                    if count > max {
                        max = count
                    }
                    count = 1;
                }
            }
        }
        if count > max {
            count
        } else {
            max
        }
    }
}

impl Accumulator for ActiveLongest {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        Ok(vec![ScalarValue::from(self.n)])
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        let value = &values[0];
        if let ScalarValue::Utf8(e) = value {
            e.iter()
                .map(|v| {
                    let ts = DateTime::parse_from_rfc2822(v).unwrap().timestamp();
                    self.data.push(ts / 86400);
                })
                .collect()
        };
        self.data.sort_unstable();
        self.n = self.calc_longest(&self.data);

        Ok(())
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        let state = &states[0];
        if let ScalarValue::Int64(Some(n)) = state {
            self.n += n;
        };
        Ok(())
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        Ok(ScalarValue::from(self.n))
    }
}

#[cfg(test)]
mod tests {
    use crate::register_udf::ActiveLongest;

    #[test]
    fn it_works() {
        let active_longest = ActiveLongest::new();
        let data = &vec![];
        assert_eq!(0, active_longest.calc_longest(data));

        let data = &vec![1];
        assert_eq!(1, active_longest.calc_longest(data));

        let data = &[1, 2];
        assert_eq!(2, active_longest.calc_longest(data));

        let data = &[1, 2, 3, 4];
        assert_eq!(4, active_longest.calc_longest(data));

        let data = &[1, 2, 3, 4, 8, 9, 20, 21, 22, 23, 24];
        assert_eq!(5, active_longest.calc_longest(data));
    }
}
