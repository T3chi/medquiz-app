use medquiz::db::Database;

fn main() -> anyhow::Result<()> {
    let db = Database::open()?;
    let report = db.backfill_question_metadata()?;

    println!("Backfill complete:");
    println!("  Questions updated: {}", report.updated);
    println!("  Subject tags added: {}", report.subjects_added);
    println!("  Citations added: {}", report.citations_added);
    println!("  Citations enriched: {}", report.citations_enriched);
    println!("  Citations still missing: {}", report.citations_missing);
    println!("  Skipped (no source): {}", report.skipped);

    Ok(())
}