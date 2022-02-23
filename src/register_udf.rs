use chrono::prelude::*;
use datafusion::{
    arrow::{
        array::{ArrayRef, StringArray, UInt64Array},
        datatypes::DataType,
    },
    physical_plan::{
        functions::{make_scalar_function, Volatility},
        udf::ScalarUDF,
    },
    prelude::*,
};
use std::sync::Arc;

pub fn udf_year() -> ScalarUDF {
    let year = |args: &[ArrayRef]| {
        let base = &args[0].as_any().downcast_ref::<StringArray>().unwrap();

        let array = base
            .iter()
            .map(|x| x.unwrap())
            .map(|x| Some(DateTime::parse_from_rfc2822(x).unwrap().year() as u64))
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
            .map(|x| x.unwrap())
            .map(|x| Some(DateTime::parse_from_rfc2822(x).unwrap().month() as u64))
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
            .map(|x| x.unwrap())
            .map(|x| {
                Some(
                    DateTime::parse_from_rfc2822(x)
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
            .map(|x| x.unwrap())
            .map(|x| Some(DateTime::parse_from_rfc2822(x).unwrap().hour() as u64))
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
            .map(|x| x.unwrap())
            .map(|x| {
                Some(
                    DateTime::parse_from_rfc2822(x)
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
