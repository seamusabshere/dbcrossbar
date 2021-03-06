//! Implementation of `write_local_data` for Redshift.

use super::RedshiftLocator;
use crate::common::*;
use crate::drivers::s3::find_s3_temp_dir;
use crate::tokio_glue::ConsumeWithParallelism;

/// Implementation of `write_local_data`, but as a real `async` function.
pub(crate) async fn write_local_data_helper(
    ctx: Context,
    dest: RedshiftLocator,
    data: BoxStream<CsvStream>,
    shared_args: SharedArguments<Unverified>,
    dest_args: DestinationArguments<Unverified>,
) -> Result<BoxStream<BoxFuture<BoxLocator>>> {
    // Build a temporary location.
    let shared_args_v = shared_args.clone().verify(RedshiftLocator::features())?;
    let s3_temp = find_s3_temp_dir(shared_args_v.temporary_storage())?;
    let s3_dest_args = DestinationArguments::for_temporary();
    let s3_source_args = SourceArguments::for_temporary();

    // Copy to a temporary s3:// location.
    let to_temp_ctx = ctx.child(o!("to_temp" => s3_temp.to_string()));
    let result_stream = s3_temp
        .write_local_data(to_temp_ctx, data, shared_args.clone(), s3_dest_args)
        .await?;

    // Wait for all s3:// uploads to finish with controllable parallelism.
    //
    // TODO: This duplicates our top-level `cp` code and we need to implement
    // the same rules for picking a good argument to `consume_with_parallelism`
    // and not just hard code our parallelism.
    result_stream
        .consume_with_parallelism(shared_args_v.max_streams())
        .await?;

    // Load from s3:// to Redshift.
    let from_temp_ctx = ctx.child(o!("from_temp" => s3_temp.to_string()));
    dest.write_remote_data(
        from_temp_ctx,
        Box::new(s3_temp),
        shared_args,
        s3_source_args,
        dest_args,
    )
    .await?;

    // We don't need any parallelism after the Redshift step, so just return
    // a stream containing a single future.
    let fut = async { Ok(dest.boxed()) }.boxed();
    Ok(box_stream_once(Ok(fut)))
}
