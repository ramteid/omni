pub const FIREFLIES_GRAPHQL_URL: &str = "https://api.fireflies.ai/graphql";
pub const BATCH_SIZE: i32 = 50;

pub const TRANSCRIPTS_QUERY: &str = r#"
query GetTranscripts($limit: Int!, $skip: Int!, $fromDate: DateTime) {
  transcripts(limit: $limit, skip: $skip, fromDate: $fromDate) {
    id
    title
    date
    duration
    organizer_email
    participants
    transcript_url
    sentences {
      speaker_name
      text
      start_time
      end_time
    }
    summary {
      keywords
      action_items
      outline
      overview
      shorthand_bullet
    }
  }
}
"#;
