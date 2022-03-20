use crate::config;
use chrono::{prelude::*, Duration};
use datafusion::{
    arrow::{
        array,
        array::ArrayRef,
        datatypes::{DataType, Field},
    },
    error::{DataFusionError, Result},
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
use std::sync::Arc;

lazy_static! {
    /// udf 函数集合
    static ref UDFS: Vec<fn() -> ScalarUDF> = vec![
        udf_year,
        udf_month,
        udf_weekday,
        udf_weeknum,
        udf_hour,
        udf_period,
        udf_timestamp,
        udf_timezone,
        udf_duration,
        udf_timestamp_rfc3339,
    ];

    /// udaf 函数集合
    static ref UDAFS: Vec<fn() -> AggregateUDF> = vec![
        udaf_active_longest_count,
        udaf_active_longest_start,
        udaf_active_longest_end,
    ];
}

const ERROR_DATEDATE_MISMATCHED: &str = "Mismatched: except rfc2882 datetime string";

/// 计算给定时间的年份
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: 2021
/// ```
fn udf_year() -> ScalarUDF {
    let year = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        }

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.year()),
                Err(_) => None,
            })
            .collect::<array::Int32Array>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let year = make_scalar_function(year);
    create_udf(
        "year",
        vec![DataType::Utf8],
        Arc::new(DataType::Int32),
        Volatility::Immutable,
        year,
    )
}

/// 计算给定时间的月份
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: 10
/// ```
fn udf_month() -> ScalarUDF {
    let month = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        }

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.month()),
                Err(_) => None,
            })
            .collect::<array::UInt32Array>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let month = make_scalar_function(month);
    create_udf(
        "month",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt32),
        Volatility::Immutable,
        month,
    )
}

/// 计算给定时间的星期字符
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: "Mon"
/// ```
fn udf_weekday() -> ScalarUDF {
    let weekday = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.weekday().to_string()),
                Err(_) => None,
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

/// 计算给定时间的星期数字
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: 0
/// ```
fn udf_weeknum() -> ScalarUDF {
    let week = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.weekday().num_days_from_monday()),
                Err(_) => None,
            })
            .collect::<array::UInt32Array>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let week = make_scalar_function(week);
    create_udf(
        "weeknum",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt32),
        Volatility::Immutable,
        week,
    )
}

/// 计算给定时间的小时数
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: 14
/// ```
fn udf_hour() -> ScalarUDF {
    let hour = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.hour()),
                Err(_) => None,
            })
            .collect::<array::UInt32Array>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let hour = make_scalar_function(hour);
    create_udf(
        "hour",
        vec![DataType::Utf8],
        Arc::new(DataType::UInt32),
        Volatility::Immutable,
        hour,
    )
}

/// 计算给定时间的状态（午夜、早上、下午以及晚上）
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: "Afternoon"
/// ```
/// `hour`  |  `[0, 8)`  | `[8, 12)` |  `[12, 18)` | `[18, 24)`
/// ------- | ---------- | --------- | ----------- | ---------
/// `period`| `Midnight` | `Morning` | `Afternoon` | `Evening`
fn udf_period() -> ScalarUDF {
    let period = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => {
                    let s = match t.hour() {
                        0..=7 => String::from("Midnight"),
                        8..=11 => String::from("Morning"),
                        12..=18 => String::from("Afternoon"),
                        19..=23 => String::from("Evening"),
                        _ => unreachable!(),
                    };
                    Some(s)
                }
                Err(_) => None,
            })
            .collect::<array::StringArray>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let period = make_scalar_function(period);
    create_udf(
        "period",
        vec![DataType::Utf8],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        period,
    )
}

/// 计算给定时间的 Unix 时间戳
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: 1636960758
/// ```
fn udf_timestamp() -> ScalarUDF {
    let timestamp = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.timestamp()),
                Err(_) => None,
            })
            .collect::<array::Int64Array>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let timestamp = make_scalar_function(timestamp);
    create_udf(
        "timestamp",
        vec![DataType::Utf8],
        Arc::new(DataType::Int64),
        Volatility::Immutable,
        timestamp,
    )
}

/// 计算给定时间的时区
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: "+07:00"
/// ```
fn udf_timezone() -> ScalarUDF {
    let timezone = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::StringArray>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        }

        let array = base
            .unwrap()
            .iter()
            .map(|x| match DateTime::parse_from_rfc3339(x.unwrap()) {
                Ok(t) => Some(t.timezone().to_string()),
                Err(_) => None,
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

/// 计算给定时间到现在时间的长度
///
/// # Example
/// ```rust
/// input<arg1: unix timestamp>: 1647272093
/// output: "30hours 2minutes"
/// ```
fn udf_duration() -> ScalarUDF {
    let duration = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::Int64Array>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| {
                let t = Utc::now().timestamp() - x.unwrap();
                Some(humantime::format_duration(Duration::seconds(t).to_std().unwrap()).to_string())
            })
            .collect::<array::StringArray>();

        Ok(Arc::new(array) as array::ArrayRef)
    };

    let duration = make_scalar_function(duration);
    create_udf(
        "duration",
        vec![DataType::Int64],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        duration,
    )
}

/// 格式化时间戳时间
///
/// # Example
/// ```rust
/// input<arg1: unix timestamp, arg2: String>: 1647272093
/// output: "2021-10-12T14:20:50.52+07:00"
/// ```
fn udf_timestamp_rfc3339() -> ScalarUDF {
    let date = |args: &[array::ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<array::Int64Array>();
        if base.is_none() {
            return Err(DataFusionError::Execution(String::from(
                ERROR_DATEDATE_MISMATCHED,
            )));
        };

        let array = base
            .unwrap()
            .iter()
            .map(|x| Some(Utc.timestamp(x.unwrap(), 0).to_rfc3339().to_string()))
            .collect::<array::StringArray>();
        Ok(Arc::new(array) as array::ArrayRef)
    };

    let date = make_scalar_function(date);
    create_udf(
        "timestamp_rfc3339",
        vec![DataType::Int64],
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        date,
    )
}

/// 计算最大连续多少天有提交记录
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: n
/// ```
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

/// 计算最大连续提交天数的起始时间
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>:"2021-10-12T14:20:50.52+07:00"
/// output: "2021-10-12"
/// ```
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

/// 计算最大连续提交天数的结束时间
///
/// # Example
/// ```rust
/// input<arg1: rfc3339>: "2021-10-12T14:20:50.52+07:00"
/// output: "2021-10-12"
/// ```
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

/// sql 查询执行器
pub struct Executor;

impl Executor {
    pub async fn create_context(config: Vec<config::Execution>) -> Result<ExecutionContext> {
        let mut ctx = ExecutionContext::new();
        for udf in UDFS.iter() {
            ctx.register_udf(udf());
        }
        for udaf in UDAFS.iter() {
            ctx.register_udaf(udaf())
        }

        for c in config {
            ctx.register_csv(&c.table_name, &c.file, CsvReadOptions::new())
                .await?;
        }
        Ok(ctx)
    }
}

/// 所有时间输入类型的 Accumulator 的基类
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

    /// 定义如何更新数据
    fn update(&mut self, values: &[ScalarValue]) -> Result<()> {
        let value = &values[0];
        if let ScalarValue::Utf8(e) = value {
            e.iter()
                .map(|v| {
                    let ts = DateTime::parse_from_rfc3339(v).unwrap().timestamp();
                    self.data.push(ts);
                })
                .collect()
        };
        Ok(())
    }

    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        if values.is_empty() {
            return Ok(());
        };
        (0..values[0].len()).try_for_each(|index| {
            let v = values
                .iter()
                .map(|array| ScalarValue::try_from_array(array, index))
                .collect::<Result<Vec<_>>>()?;
            self.update(&v)
        })
    }
}

#[derive(Debug, Clone)]
enum ActiveLongestType {
    /// 最大连续天数
    Count,

    /// 起始时间
    Start,

    /// 结束时间
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
    fn new() -> Self {
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

    fn merge_index<I: Into<u8>>(&mut self, states: &[ScalarValue], index: I) -> Result<()> {
        for state in states {
            if let ScalarValue::List(Some(values), _) = state {
                for v in values.iter() {
                    if let ScalarValue::Int64(i) = v {
                        self.tla.data.push(i.unwrap());
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

    fn merge_batch<I: Into<u8> + Clone>(
        &mut self,
        states: &[ArrayRef],
        merge_index: I,
    ) -> Result<()> {
        if states.is_empty() {
            return Ok(());
        };
        (0..states[0].len()).try_for_each(|index| {
            let v = states
                .iter()
                .map(|array| ScalarValue::try_from_array(array, index))
                .collect::<Result<Vec<_>>>()?;
            self.merge_index(&v, merge_index.clone())
        })
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

    fn evaluate(&self) -> Result<ScalarValue> {
        Ok(ScalarValue::from(self.al.tla.n))
    }

    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        self.al.tla.update_batch(values)
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        self.al.merge_batch(states, ActiveLongestType::Count)
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

    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        self.al.tla.update_batch(values)
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        self.al.merge_batch(states, self.index)
    }

    fn evaluate(&self) -> Result<ScalarValue> {
        let s = Utc
            .timestamp(self.al.tla.n, 0)
            .format("%Y-%m-%d")
            .to_string();
        Ok(ScalarValue::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use datafusion::{
        arrow,
        arrow::{array::Array, datatypes::Schema, record_batch::RecordBatch},
        datasource::MemTable,
    };

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

    fn get_datetime_context() -> ExecutionContext {
        let mut ctx = ExecutionContext::new();
        let datetime_array: array::LargeStringArray = vec![
            "2021-10-12T14:20:50.52+08:00",
            "2021-10-13T08:20:50.52+08:00",
            "2020-01-02T22:20:50.52+07:00",
            "2020-03-03T11:39:50.52+07:00",
        ]
        .into_iter()
        .map(Some)
        .collect();
        let datetime_array = Arc::new(datetime_array);
        let schema = Arc::new(Schema::new(vec![Field::new(
            "datetime",
            datetime_array.data_type().clone(),
            false,
        )]));

        for udf in UDFS.iter() {
            ctx.register_udf(udf());
        }
        for udaf in UDAFS.iter() {
            ctx.register_udaf(udaf())
        }

        let batch = RecordBatch::try_new(schema.clone(), vec![datetime_array]).unwrap();
        let provider = MemTable::try_new(schema.clone(), vec![vec![batch]]).unwrap();
        ctx.register_table("repo", Arc::new(provider)).unwrap();
        ctx
    }

    #[tokio::test]
    async fn test_udf_year() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select year(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+---------------------+",
            "| year(repo.datetime) |",
            "+---------------------+",
            "| 2020                |",
            "| 2020                |",
            "| 2021                |",
            "| 2021                |",
            "+---------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_month() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select month(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+----------------------+",
            "| month(repo.datetime) |",
            "+----------------------+",
            "| 1                    |",
            "| 10                   |",
            "| 10                   |",
            "| 3                    |",
            "+----------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_weekday() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select weekday(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+------------------------+",
            "| weekday(repo.datetime) |",
            "+------------------------+",
            "| Thu                    |",
            "| Tue                    |",
            "| Tue                    |",
            "| Wed                    |",
            "+------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_weeknum() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select weeknum(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+------------------------+",
            "| weeknum(repo.datetime) |",
            "+------------------------+",
            "| 1                      |",
            "| 1                      |",
            "| 2                      |",
            "| 3                      |",
            "+------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_hour() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select hour(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+---------------------+",
            "| hour(repo.datetime) |",
            "+---------------------+",
            "| 14                  |",
            "| 8                   |",
            "| 22                  |",
            "| 11                  |",
            "+---------------------+",
        ];
        datafusion::assert_batches_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_period() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select period(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+-----------------------+",
            "| period(repo.datetime) |",
            "+-----------------------+",
            "| Afternoon             |",
            "| Evening               |",
            "| Morning               |",
            "| Morning               |",
            "+-----------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_timestamp() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select timestamp(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+--------------------------+",
            "| timestamp(repo.datetime) |",
            "+--------------------------+",
            "| 1577978450               |",
            "| 1583210390               |",
            "| 1634019650               |",
            "| 1634084450               |",
            "+--------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_timezone() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select timezone(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+-------------------------+",
            "| timezone(repo.datetime) |",
            "+-------------------------+",
            "| +07:00                  |",
            "| +07:00                  |",
            "| +08:00                  |",
            "| +08:00                  |",
            "+-------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udf_timestamp_rfc3339() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select timestamp_rfc3339(1647272093) as t from repo limit 1;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+---------------------------+",
            "| t                         |",
            "+---------------------------+",
            "| 2022-03-14T15:34:53+00:00 |",
            "+---------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udaf_active_longest_count() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select active_longest_count(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+-------------------------------------+",
            "| active_longest_count(repo.datetime) |",
            "+-------------------------------------+",
            "| 2                                   |",
            "+-------------------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udaf_active_longest_start() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select active_longest_start(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+-------------------------------------+",
            "| active_longest_start(repo.datetime) |",
            "+-------------------------------------+",
            "| 2021-10-12                          |",
            "+-------------------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }

    #[tokio::test]
    async fn test_udaf_active_longest_end() {
        let mut ctx = get_datetime_context();
        let result: Vec<RecordBatch> = ctx
            .sql("select active_longest_end(datetime) from repo;")
            .await
            .unwrap()
            .collect()
            .await
            .unwrap();

        let expected = vec![
            "+-----------------------------------+",
            "| active_longest_end(repo.datetime) |",
            "+-----------------------------------+",
            "| 2021-10-13                        |",
            "+-----------------------------------+",
        ];
        datafusion::assert_batches_sorted_eq!(expected, &result);
    }
}
