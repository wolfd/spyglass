pub mod parser;
pub mod to_polars;
pub use parser::parse;
pub use to_polars::to_polars_expr;

pub use polars::error::PolarsResult;
pub use polars::prelude::DataType;
pub use polars::prelude::PolarsError;
pub use polars::prelude::{LazyFrame, ListToStructArgs, ToStruct};

use anyhow::Result;
use std::ffi::OsStr;
use std::path::PathBuf;

use polars::prelude::*;

pub fn read_data(path: PathBuf) -> PolarsResult<LazyFrame> {
    if path.extension() == Some(OsStr::new("mcap")) {
        return PolarsResult::Ok(
            // this is bad
            mcap_polars::hacky_as_hell_mcap_to_dataframe(&path)
                .map(|df| df.lazy())
                .unwrap(),
        );
    }
    LazyFrame::scan_parquet(path, ScanArgsParquet::default())
}

pub struct Trace {
    pub name: String,
    pub data: Vec<f64>,
}

pub fn eval(df: &LazyFrame, expr: &str) -> Result<Vec<Trace>> {
    let slang_expr = crate::parse(expr)?;
    let polars_expr = crate::to_polars_expr(&slang_expr)?;

    let data = df
        .clone() // TODO: remove clone
        .lazy()
        .select([polars_expr])
        .collect()?;

    let series = data
        .get_columns()
        .into_iter()
        .next()
        .ok_or(anyhow::anyhow!("No data"))?
        .as_series()
        .ok_or(anyhow::anyhow!("Can't make series from column"))?;

    let splat_series = match series.dtype() {
        DataType::List(_) => unnest_series(series)?,
        DataType::Struct(_) => unnest_series(series)?,
        _ => vec![series.clone()],
    };

    splat_series
        .iter()
        .enumerate()
        .map(|(index, s)| {
            let data = s
                .cast(&DataType::Float64)?
                .f64()?
                .to_vec_null_aware()
                .left()
                .ok_or(anyhow::anyhow!("Can't convert to f64"))?;
            Ok(Trace {
                name: if splat_series.len() == 1 {
                    expr.to_owned()
                } else {
                    format!("{}[{}]", expr.to_owned(), index)
                },
                data,
            })
        })
        .collect()
}

fn unnest_series(series: &Series) -> Result<Vec<Series>> {
    let structs = match series.dtype() {
        DataType::List(_) => series.list()?.to_struct(&ListToStructArgs::InferWidth {
            infer_field_strategy: polars::prelude::ListToStructWidthStrategy::FirstNonNull,
            get_index_name: None,
            max_fields: 100,
        })?,
        // TODO: really don't clone
        DataType::Struct(_inner) => series.struct_()?.clone(),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported type, cannot unnest: {:?}",
                series.dtype()
            ));
        }
    };

    Ok(structs.fields_as_series())
}
