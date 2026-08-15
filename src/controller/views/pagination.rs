use serde::{Deserialize, Serialize};

/// A page of results plus its pagination metadata.
///
/// The `info` field is named `pagination` on the wire. That rename applies to
/// **both** directions: it used to be `rename(serialize = ..)`, so `Pager`
/// serialized to `{"results": .., "pagination": ..}` and then failed to
/// deserialize its own output, reporting a missing `info` field. Nothing else
/// produces that shape, so no wire format changed — only the derive that had
/// been unusable since it was written.
#[derive(Debug, Deserialize, Serialize)]
pub struct Pager<T> {
    pub results: T,

    #[serde(rename = "pagination")]
    pub info: PagerMeta,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PagerMeta {
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
    pub total_items: u64,
}

impl<T> Pager<T> {
    #[must_use]
    pub const fn new(results: T, meta: PagerMeta) -> Self {
        Self {
            results,
            info: meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Pager<Vec<i32>> {
        Pager::new(
            vec![1, 2, 3],
            PagerMeta {
                page: 1,
                page_size: 10,
                total_pages: 1,
                total_items: 3,
            },
        )
    }

    /// The wire shape is public API — pin it, so the rename above can never
    /// be "cleaned up" into a different response body.
    #[test]
    fn serializes_to_results_and_pagination() {
        let json = serde_json::to_value(sample()).expect("Pager serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "results": [1, 2, 3],
                "pagination": {
                    "page": 1,
                    "page_size": 10,
                    "total_pages": 1,
                    "total_items": 3
                }
            })
        );
    }

    /// `Pager` derives `Deserialize`, so a client can use it to read a Loco
    /// response. It could not: the serialize-only rename emitted `pagination`
    /// while the deserializer looked for `info`.
    #[test]
    fn deserializes_what_it_serializes() {
        let json = serde_json::to_string(&sample()).expect("Pager serializes");
        let back: Pager<Vec<i32>> =
            serde_json::from_str(&json).expect("Pager should read its own output");

        assert_eq!(back.results, vec![1, 2, 3]);
        assert_eq!(back.info.page, 1);
        assert_eq!(back.info.page_size, 10);
        assert_eq!(back.info.total_pages, 1);
        assert_eq!(back.info.total_items, 3);
    }
}
