use chrono::{prelude::*, Duration};
use datafusion::{
    arrow::{
        array,
        array::ArrayRef,
        datatypes::{DataType, Field},
    },
    error::Result,
    logical_plan::create_udaf,
    physical_plan::{
        functions::{make_scalar_function, Volatility},
        udaf::AggregateUDF,
        udf::ScalarUDF,
        Accumulator,
    },
    prelude::*,
    scalar::ScalarValue,
};
use lazy_static::lazy_static;
use std::{collections::HashSet, sync::Arc};

lazy_static! {
    static ref UDFS: Vec<fn() -> ScalarUDF> = vec![
        udf_year,
        udf_month,
        udf_weekday,
        udf_hour,
        udf_timestamp,
        udf_timezone,
        udf_duration,
        udf_time_format,
    ];
    static ref UDAFS: Vec<fn() -> AggregateUDF> = vec![
        udaf_active_days,
        udaf_active_longest_count,
        udaf_active_longest_start,
        udaf_active_longest_end,
    ];
}

pub fn create_execution_context() -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    for udf in UDFS.iter() {
        ctx.register_udf(udf());
    }
    for udaf in UDAFS.iter() {
        ctx.register_udaf(udaf())
    }
    ctx
}

/// udf_year 计算给定时间的年份
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: 2021
fn udf_year() -> ScalarUDF {
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

/// udf_year 计算给定时间的月份
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: 11
fn udf_month() -> ScalarUDF {
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

/// udf_year 计算给定时间的月份
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: "Mon"
fn udf_weekday() -> ScalarUDF {
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

/// udf_year 计算给定时间的小时数
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: 15
fn udf_hour() -> ScalarUDF {
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

/// udf_year 计算给定时间的 Unix 时间戳
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: 1636960758
fn udf_timestamp() -> ScalarUDF {
    let timestamp = |args: &[array::ArrayRef]| {
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
                        .timestamp() as u64,
                )
            })
            .collect::<array::UInt64Array>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let timestamp = make_scalar_function(timestamp);
    create_udf(
        "timestamp",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt64),
        Volatility::Immutable,
        timestamp,
    )
}

/// udf_year 计算给定时间到现在时间的长度
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: "30hours 2minutes"
fn udf_duration() -> ScalarUDF {
    let duration = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::UInt64Array>()
            .unwrap();
        let array = base
            .iter()
            .map(|x| {
                let t = Utc::now().timestamp() - x.unwrap() as i64;
                Some(humantime::format_duration(Duration::seconds(t).to_std().unwrap()).to_string())
            })
            .collect::<array::StringArray>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let duration = make_scalar_function(duration);
    create_udf(
        "duration",
        vec![DataType::UInt64],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        duration,
    )
}

/// udf_year 计算给定时间的时区
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: "+08:00"
fn udf_timezone() -> ScalarUDF {
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

/// udf_year 格式化时间
///
/// input<arg1: rfc2822, arg2: String>: ("Mon, 15 Nov 2021 15:19:18 +0800", "%Y-%m-%d %H:%M:%S")
/// output: "2021-11-15 15:19:18"
fn udf_time_format() -> ScalarUDF {
    let date = |args: &[array::ArrayRef]| {
        let base = &args[0]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
        let format = &args[1]
            .as_any()
            .downcast_ref::<array::StringArray>()
            .unwrap();
        let format = format.value(0);
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
        "time_format",
        vec![DataType::Utf8, DataType::Utf8],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        date,
    )
}

/// udaf_active_days 计算某天 commits 数量
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: n
fn udaf_active_days() -> AggregateUDF {
    create_udaf(
        "active_days",
        DataType::Utf8,
        Arc::new(DataType::Int64),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveDays::new()))),
        Arc::new(vec![DataType::Int64]),
    )
}

/// udaf_active_longest_count 计算最大连续多少天有提交记录
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: n
fn udaf_active_longest_count() -> AggregateUDF {
    create_udaf(
        "active_longest_count",
        DataType::Utf8,
        Arc::new(DataType::Int64),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveLongestCount::new()))),
        Arc::new(vec![DataType::List(Box::new(Field::new(
            "item",
            DataType::Int64,
            true,
        )))]),
    )
}

/// udaf_active_longest_count 计算最大连续提交天数的起始时间
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: "2020-01-02"
fn udaf_active_longest_start() -> AggregateUDF {
    create_udaf(
        "active_longest_start",
        DataType::Utf8,
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveLongestTime::new(ActiveLongestType::Start)))),
        Arc::new(vec![DataType::List(Box::new(Field::new(
            "item",
            DataType::Int64,
            true,
        )))]),
    )
}

/// udaf_active_longest_count 计算最大连续提交天数的结束时间
///
/// input<arg1: rfc2822>: "Mon, 15 Nov 2021 15:19:18 +0800"
/// output: "2020-01-05"
fn udaf_active_longest_end() -> AggregateUDF {
    create_udaf(
        "active_longest_end",
        DataType::Utf8,
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(|| Ok(Box::new(ActiveLongestTime::new(ActiveLongestType::End)))),
        Arc::new(vec![DataType::List(Box::new(Field::new(
            "item",
            DataType::Int64,
            true,
        )))]),
    )
}

#[derive(Debug)]
struct TimeInputAccumulator {
    data: Vec<i64>,
    n: i64,
}

impl TimeInputAccumulator {
    fn new() -> Self {
        Self { data: vec![], n: 0 }
    }

    fn state(&self) -> Result<Vec<ScalarValue>> {
        let mut values = Box::new(vec![]);
        for d in self.data.iter() {
            values.push(ScalarValue::from(*d as i64))
        }

        let values = ScalarValue::List(Some(values), Box::new(DataType::Int64));
        Ok(vec![values])
    }

    fn gather(&mut self, states: &[ScalarValue]) {
        for state in states {
            if let ScalarValue::List(Some(values), _) = state {
                for v in values.iter() {
                    if let ScalarValue::Int64(i) = v {
                        self.data.push(i.unwrap())
                    }
                }
            };
        }
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

    #[allow(dead_code)]
    fn merge(&mut self, _: &[ScalarValue]) -> Result<()> {
        panic!("implement me")
    }

    #[allow(dead_code)]
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
}

#[derive(Debug)]
struct ActiveDays {
    tla: TimeInputAccumulator,
}

impl ActiveDays {
    pub fn new() -> Self {
        Self {
            tla: TimeInputAccumulator::new(),
        }
    }
}

impl Accumulator for ActiveDays {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        self.tla.state()
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        self.tla.update(values)
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        self.tla.gather(states);
        let mut set: HashSet<String> = HashSet::new();
        for v in self.tla.data.iter() {
            let s = Utc.timestamp(*v, 0).format("%Y-%m-%d").to_string();
            set.insert(s);
        }

        self.tla.n = set.len() as i64;
        Ok(())
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        Ok(ScalarValue::from(self.tla.n as i64))
    }
}

enum ActiveLongestType {
    Count,
    Start,
    End,
}

impl From<ActiveLongestType> for u8 {
    fn from(t: ActiveLongestType) -> Self {
        match t {
            ActiveLongestType::Count => 0,
            ActiveLongestType::Start => 1,
            ActiveLongestType::End => 2,
        }
    }
}

#[derive(Debug)]
struct ActiveLongest {
    tla: TimeInputAccumulator,
}

impl ActiveLongest {
    pub fn new() -> Self {
        Self {
            tla: TimeInputAccumulator::new(),
        }
    }

    /// calc_longest 计算提交持续天数的数量以及起止时间
    ///
    /// 采用双指针算法，时间复杂度 O(N)
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

    fn merge_index<S: Into<u8>>(&mut self, states: &[ScalarValue], index: S) -> Result<()> {
        for state in states {
            if let ScalarValue::List(Some(values), _) = state {
                for v in values.iter() {
                    if let ScalarValue::Int64(i) = v {
                        self.tla.data.push(i.unwrap())
                    }
                }
            };
        }

        self.tla.data.sort_unstable();
        let ret = self.calc_longest(&self.tla.data, 3600 * 24);
        match index.into() {
            0 => self.tla.n = ret.0,
            1 => self.tla.n = ret.1,
            2 => self.tla.n = ret.2,
            _ => (),
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ActiveLongestCount {
    al: ActiveLongest,
}

impl ActiveLongestCount {
    fn new() -> Self {
        Self {
            al: ActiveLongest::new(),
        }
    }
}

impl Accumulator for ActiveLongestCount {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        self.al.tla.state()
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        self.al.tla.update(values)
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        self.al.merge_index(states, ActiveLongestType::Count)
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        Ok(ScalarValue::from(self.al.tla.n))
    }
}

#[derive(Debug)]
struct ActiveLongestTime {
    al: ActiveLongest,
    index: u8,
}

impl ActiveLongestTime {
    fn new<I: Into<u8>>(index: I) -> Self {
        Self {
            al: ActiveLongest::new(),
            index: index.into(),
        }
    }
}

impl Accumulator for ActiveLongestTime {
    fn state(&self) -> Result<Vec<ScalarValue>> {
        self.al.tla.state()
    }

    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        self.al.tla.update(values)
    }

    fn merge(&mut self, states: &[ScalarValue]) -> Result<()> {
        self.al.merge_index(states, self.index)
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        let s = Utc.timestamp(self.al.tla.n, 0) + Duration::days(1);
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
