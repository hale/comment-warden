//! TRIPWIRE: this module's raw strings are load-bearing test fixtures for the comment-tagging tool itself — do not "clean up" the // lines inside them

/// This doc comment carries no tag and must still survive stripping — it is
/// protected because async-graphql copies it into the SDL, not because of a tag.
#[derive(SimpleObject)]
pub struct BuiltQuery {
    sql: String,
}

pub fn build_query() -> &'static str {
    r#"
    // this looks like a comment but is SQL-adjacent fixture text inside a raw string
    #!/not/a/real/shebang/either
    SELECT 1;
    "#
}

fn describe() -> &'static str {
    "see https://example.com // this is part of the string, not a comment"
}

// this line explains nothing that the code doesn't already say

// TRIPWIRE: a tagged line keeps the continuation lines below it
// this second line is part of that note and survives with the tag above it

fn main() {
    let _ = build_query();
    let _ = describe(); // trailing untagged comment
}
