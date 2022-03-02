use chrono::prelude::*;
use chrono::Duration;
use datafusion::arrow::array::ArrayRef;
use datafusion::arrow::datatypes::Field;
use datafusion::logical_plan::create_udaf;
use datafusion::physical_plan::udaf::AggregateUDF;
use datafusion::physical_plan::Accumulator;
use datafusion::scalar::ScalarValue;
use datafusion::{
    arrow::{array, datatypes::DataType},
    error::Result,
    physical_plan::{
        functions::{make_scalar_function, Volatility},
        udf::ScalarUDF,
    },
    prelude::*,
};
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::sync::Arc;

lazy_static! {
    pub(crate) static ref UDFS: Vec<fn() -> ScalarUDF> = vec![
        udf_year,
        udf_month,
        udf_weekday,
        udf_hour,
        udf_timezone,
        udf_date_day,
        udf_date_month,
    ];
    pub(crate) static ref UDAFS: Vec<fn() -> AggregateUDF> = vec![
        udf_active_days,
        udf_active_longest,
        udf_active_longest_start,
        udf_active_longest_end,
    ];
}

pub fn udf_year() -> ScalarUDF {
    let year = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
        let array = base
            .iter()
            .map(|x| Some(DateTime::parse_from_rfc2822(x.unwrap()).unwrap().year() as u64))
            .collect::<array::UInt64Array>();

        Ok(Arc::new(array) as array::ArrayRef)
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
    let month = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
        let array = base
            .iter()
            .map(|x| Some(DateTime::parse_from_rfc2822(x.unwrap()).unwrap().month() as u64))
            .collect::<array::UInt64Array>();

        Ok(Arc::new(array) as array::ArrayRef)
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
    let weekday = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
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
            .collect::<array::StringArray>();

        Ok(Arc::new(array) as array::ArrayRef)
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
    let hour = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
        let array = base
            .iter()
            .map(|x| Some(DateTime::parse_from_rfc2822(x.unwrap()).unwrap().hour() as u64))
            .collect::<array::UInt64Array>();

        Ok(Arc::new(array) as array::ArrayRef)
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
    let timezone = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
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
            .collect::<array::StringArray>();

        Ok(Arc::new(array) as array::ArrayRef)
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
    let date = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
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
            .collect::<array::StringArray>();

        Ok(Arc::new(array) as array::ArrayRef)
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
    data: Vec<i64>,
    n: i64,
}

impl ActiveDays {
    pub fn new() -> Self {
        Self { data: vec![], n: 0 }
    }
}

impl Accumulator for ActiveDays {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        let mut values = Box::new(vec![]);
        for d in self.data.iter() {
            values.push(ScalarValue::from(*d as i64))
        }

        let values = ScalarValue::List(Some(values), Box::new(DataType::Int64));
        Ok(vec![values])
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
        Arc::new(vec![DataType::List(Box::new(Field::new(
            "item",
            DataType::Int64,
            true,
        )))]),
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

    fn calc_longest(&self, data: &[i64], ratio: i64) -> (i64, i64, i64) {
        if data.is_empty() {
            return (0, 0, 0);
        }
        if data.len() <= 1 {
            return (1, data[0], data[0]);
        }

        let mut count: i64 = 1;
        let mut max: i64 = 0;
        let mut l: usize = 0;
        let mut r: usize = 0;
        let mut start: usize = 0;
        let mut end: usize = 0;
        for i in 0..data.len() - 1 {
            let k = data[i + 1] / ratio - data[i] / ratio;
            match k {
                0 | 1 => {
                    r = i + 1;
                    count += k;
                }
                _ => {
                    if count > max {
                        max = count;
                        (start, end) = (l, r);
                    }
                    l = i + 1;
                    count = 1;
                }
            }
        }
        if count > max {
            (count, data[l], data[r])
        } else {
            (max, data[start], data[end])
        }
    }

    fn merge_index(&mut self, states: &[ScalarValue], index: u8) -> Result<()> {
        for state in states {
            if let ScalarValue::List(Some(values), _) = state {
                for v in values.iter() {
                    if let ScalarValue::Int64(i) = v {
                        self.data.push(i.unwrap())
                    }
                }
            };
        }

        self.data.sort_unstable();
        let ret = self.calc_longest(&self.data, 3600 * 24);
        match index {
            0 => self.n = ret.0,
            1 => self.n = ret.1,
            2 => self.n = ret.2,
            _ => (),
        }
        Ok(())
    }
}

impl Accumulator for ActiveLongest {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        let mut values = Box::new(vec![]);
        for d in self.data.iter() {
            values.push(ScalarValue::from(*d as i64))
        }

        let values = ScalarValue::List(Some(values), Box::new(DataType::Int64));
        Ok(vec![values])
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        let value = &values[0];
        if let ScalarValue::Utf8(e) = value {
            e.iter()
                .map(|v| {
                    let ts = DateTime::parse_from_rfc2822(v).unwrap().timestamp();
                    self.data.push(ts);
                })
                .collect()
        };
        Ok(())
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        self.merge_index(states, 0)
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        if states.is_empty() {
            return Ok(());
        };
        let mut data = vec![];
        (0..states[0].len()).for_each(|index| {
            let v = states
                .iter()
                .map(|array| ScalarValue::try_from_array(array, index))
                .collect::<Result<Vec<_>>>()
                .unwrap();
            data.extend(v);
        });

        self.merge(&data)
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        Ok(ScalarValue::from(self.n))
    }
}

pub fn udf_active_longest_start() -> AggregateUDF {
    create_udaf(
        "active_longest_start",
        DataType::Utf8,
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveLongestTime::new(1)))),
        Arc::new(vec![DataType::List(Box::new(Field::new(
            "item",
            DataType::Int64,
            true,
        )))]),
    )
}

pub fn udf_active_longest_end() -> AggregateUDF {
    create_udaf(
        "active_longest_end",
        DataType::Utf8,
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveLongestTime::new(2)))),
        Arc::new(vec![DataType::List(Box::new(Field::new(
            "item",
            DataType::Int64,
            true,
        )))]),
    )
}

#[derive(Debug)]
struct ActiveLongestTime {
    al: ActiveLongest,
    index: u8,
}

impl ActiveLongestTime {
    fn new(index: u8) -> Self {
        Self {
            al: ActiveLongest::new(),
            index,
        }
    }
}

impl Accumulator for ActiveLongestTime {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        self.al.state()
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        self.al.update(values)
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        self.al.merge_index(states, self.index)
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        let s = Utc.timestamp(self.al.n, 0) + Duration::days(1);
        let s = s.format("%Y-%m-%d").to_string();
        Ok(ScalarValue::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_active_longest() {
        let active_longest = ActiveLongest::new();
        let data = &vec![];
        assert_eq!((0, 0, 0), active_longest.calc_longest(data, 1));

        let data = &vec![1];
        assert_eq!((1, 1, 1), active_longest.calc_longest(data, 1));

        let data = &[1, 2];
        assert_eq!((2, 1, 2), active_longest.calc_longest(data, 1));

        let data = &[1, 2, 3, 4];
        assert_eq!((4, 1, 4), active_longest.calc_longest(data, 1));

        let data = &[1, 2, 3, 4, 8, 9, 20, 21, 22, 23, 24];
        assert_eq!((5, 20, 24), active_longest.calc_longest(data, 1));

        let data = &[1, 2, 3, 4, 5, 9, 20, 21, 22, 23, 24];
        assert_eq!((5, 1, 5), active_longest.calc_longest(data, 1));
    }
}
