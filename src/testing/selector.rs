use scraper::{Html, Selector};

/// Asserts that an element matching the given CSS selector exists in the
/// provided HTML.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///           <div class="some-class">Some content here</div>
///       </body>
///   </html>"#;
/// assert_css_exists(html, ".some-class");
/// ```
///
/// # Panics
///
/// This function will panic if no element matching the selector is found in the
/// HTML.
pub fn assert_css_exists(html: &str, selector: &str) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();
    assert!(
        document.select(&parsed_selector).count() > 0,
        "Element matching selector '{selector:?}' not found"
    );
}

/// Asserts that an element matching the given CSS selector does **not** exist
/// in the provided HTML.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///           <div class="some-class">Some content here</div>
///       </body>
///   </html>"#;
/// assert_css_not_exists(html, ".nonexistent-class");
/// ```
///
/// # Panics
///
/// This function will panic if an element matching the selector is found in the
/// HTML.
pub fn assert_css_not_exists(html: &str, selector: &str) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();
    assert!(
        document.select(&parsed_selector).count() == 0,
        "Element matching selector '{selector:?}' should not exist"
    );
}

/// Asserts that the text content of an element matching the given CSS selector
/// exactly matches the expected text.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///           <h1 class="title">Welcome to Loco</h1>
///       </body>
///   </html>"#;
/// assert_css_eq(html, "h1.title", "Welcome to Loco");
/// ```
///
/// # Panics
///
/// This function will panic if the text of the found element does not match the
/// expected text.
pub fn assert_css_eq(html: &str, selector: &str, expected_text: &str) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();
    let mut found = false;

    for element in document.select(&parsed_selector) {
        let text = element.text().collect::<Vec<_>>().join("");
        if text == expected_text {
            found = true;
            break;
        }
    }

    assert!(
        found,
        "Text does not match: Expected '{expected_text:?}' but found a different value or no \
         match for selector '{selector:?}'"
    );
}

/// Asserts that an `<a>` element matching the given CSS selector has the `href`
/// attribute with the specified value.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///           <a href="https://loco.rs">Link</a>
///       </body>
///   </html>"#;
/// assert_link(html, "a", "https://loco.rs");
/// ```
///
/// # Panics
///
/// This function will panic if no `<a>` element matching the selector is found,
/// if the element does not have the `href` attribute, or if the `href`
/// attribute's value does not match the expected value.
pub fn assert_link(html: &str, selector: &str, expected_href: &str) {
    // Use `assert_attribute_eq` to check that the `href` attribute exists and
    // matches the expected value
    assert_attribute_eq(html, selector, "href", expected_href);
}

/// Asserts that an element matching the given CSS selector has the specified
/// attribute.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///           <button onclick="alert('clicked')">Loco Website</button>
///           <a href="https://loco.rs">Link</a>
///       </body>
///   </html>"#;
/// assert_attribute_exists(html, "button", "onclick");
/// assert_attribute_exists(html, "a", "href");
/// ```
///
/// # Panics
///
/// This function will panic if no element matching the selector is found, or if
/// the element does not have the specified attribute.
pub fn assert_attribute_exists(html: &str, selector: &str, attribute: &str) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();

    let mut found = false;

    for element in document.select(&parsed_selector) {
        if element.value().attr(attribute).is_some() {
            found = true;
            break;
        }
    }

    assert!(
        found,
        "Element matching selector '{selector:?}' does not have the attribute '{attribute}'"
    );
}

/// Asserts that the specified attribute of an element matching the given CSS
/// selector matches the expected value.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///           <button onclick="alert('clicked')">Loco Website</button>
///           <a href="https://loco.rs">Link</a>
///       </body>
///   </html>"#;
/// assert_attribute_exists(html, "button", "onclick");
/// assert_attribute_exists(html, "a", "href");
/// ```
///
/// # Panics
///
/// This function will panic if no element matching the selector is found, if
/// the element does not have the specified attribute, or if the attribute's
/// value does not match the expected value.
pub fn assert_attribute_eq(html: &str, selector: &str, attribute: &str, expected_value: &str) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();

    let mut found = false;

    for element in document.select(&parsed_selector) {
        if let Some(attr_value) = element.value().attr(attribute) {
            if attr_value == expected_value {
                found = true;
                break;
            }
        }
    }

    assert!(
        found,
        "Expected attribute '{attribute}' with value '{expected_value}' for selector \
         '{selector:?}', but found a different value or no value."
    );
}

/// Asserts that the number of elements matching the given CSS selector in the
/// provided HTML is exactly the expected count.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///         <ul id="posts">
///             <li>Post 1</li>
///             <li>Post 2</li>
///             <li>Post 3</li>
///         </ul>  
///       </body>
///   </html>"#;
/// assert_count(html, "ul#posts li", 3);
/// ```
///
/// # Panics
///
/// This function will panic if the number of elements matching the selector is
/// not equal to the expected count.
pub fn assert_count(html: &str, selector: &str, expected_count: usize) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();

    let count = document.select(&parsed_selector).count();

    assert!(
        count == expected_count,
        "Expected {expected_count} elements matching selector '{selector:?}', but found {count} \
         elements."
    );
}

/// Collects the text content of all elements matching the given CSS selector
/// and asserts that they match the expected text.
///
/// # Example
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///   <html>
///       <body>
///         <ul id="posts">
///             <li>Post 1</li>
///             <li>Post 2</li>
///             <li>Post 3</li>
///         </ul>  
///       </body>
///   </html>"#;
/// assert_css_eq_list(html, "ul#posts li", &["Post 1", "Post 2", "Post 3"]);
/// ```
///
/// # Panics
///
/// This function will panic if the text content of the elements does not match
/// the expected values.
pub fn assert_css_eq_list(html: &str, selector: &str, expected_texts: &[&str]) {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();

    let collected_texts: Vec<String> = document
        .select(&parsed_selector)
        .map(|element| element.text().collect::<Vec<_>>().concat())
        .collect();

    assert_eq!(
        collected_texts, expected_texts,
        "Expected texts {expected_texts:?}, but found {collected_texts:?}."
    );
}

/// Parses the given HTML string and selects the elements matching the specified
/// CSS selector.
///
/// # Examples
///
/// ```rust
/// use loco_rs::testing::prelude::*;
///
/// let html = r#"
///     <html>
///         <body>
///             <div class="item">Item 1</div>
///             <div class="item">Item 2</div>
///             <div class="item">Item 3</div>
///         </body>
///     </html>
/// "#;
/// let items = select(html, ".item");
/// assert_eq!(items, vec!["<div class=\"item\">Item 1</div>", "<div class=\"item\">Item 2</div>", "<div class=\"item\">Item 3</div>"]);
/// ```
///
/// # Panics
///
/// This function will panic when could not pase the selector
#[must_use]
pub fn select(html: &str, selector: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let parsed_selector = Selector::parse(selector).unwrap();
    document
        .select(&parsed_selector)
        .map(|element| element.html())
        .collect()
}

// Test cases
#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_html() -> &'static str {
        r#"
    <html>
        <body>
            <div class="some-class">Some content here</div>
            <div class="another-class">Another content here</div>
            <h1 class="title">Welcome to Loco</h1>
            <button onclick="alert('clicked')">Loco Website</button>
            <a href="https://loco.rs">Link</a>
            <ul id="posts">
                <li>Post 1</li>
                <li>Post 2</li>
                <li>Post 3</li>
            </ul>

            <body>
                <table id="posts_table">
                    <tr>
                        <td>Post 1</td>
                        <td>Author 1</td>
                    </tr>
                    <tr>
                        <td>Post 2</td>
                        <td>Author 2</td>
                    </tr>
                    <tr>
                        <td>Post 3</td>
                        <td>Author 3</td>
                    </tr>
                </table>
            </body>
        </body>
    </html>
    "#
    }

    #[test]
    fn test_assert_css_exists() {
        let html = setup_test_html();

        assert_css_exists(html, ".some-class");

        let result = std::panic::catch_unwind(|| {
            assert_css_exists(html, ".nonexistent-class");
        });
        assert!(result.is_err(), "Expected panic for non-existent selector");
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Element matching selector '\".nonexistent-class\"' not found"
            );
        }
    }

    #[test]
    fn test_assert_css_not_exists() {
        let html = setup_test_html();

        assert_css_not_exists(html, ".nonexistent-class");

        let result = std::panic::catch_unwind(|| {
            assert_css_not_exists(html, ".some-class");
        });
        assert!(result.is_err(), "Expected panic for non-existent selector");
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Element matching selector '\".some-class\"' should not exist"
            );
        }
    }

    #[test]
    fn test_assert_css_eq() {
        let html = setup_test_html();

        assert_css_eq(html, "h1.title", "Welcome to Loco");

        let result = std::panic::catch_unwind(|| {
            assert_css_eq(html, "h1.title", "Wrong text");
        });
        assert!(result.is_err(), "Expected panic for mismatched text");
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Text does not match: Expected '\"Wrong text\"' but found a different value or \
                  no match for selector '\"h1.title\"'"
            );
        }
    }

    #[test]
    fn test_assert_link() {
        let html = setup_test_html();

        assert_link(html, "a", "https://loco.rs");

        let result = std::panic::catch_unwind(|| {
            assert_link(html, "a", "https://nonexistent.com");
        });

        assert!(result.is_err());
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Expected attribute 'href' with value 'https://nonexistent.com' for selector \
                  '\"a\"', but found a different value or no value."
            );
        }
    }

    #[test]
    fn test_assert_attribute_exists() {
        let html = setup_test_html();

        assert_attribute_exists(html, "button", "onclick");
        assert_attribute_exists(html, "a", "href");

        let result = std::panic::catch_unwind(|| {
            assert_attribute_exists(html, "button", "href");
        });
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Element matching selector '\"button\"' does not have the attribute 'href'"
            );
        }
    }

    #[test]
    fn test_assert_attribute_eq() {
        let html = setup_test_html();
        assert_attribute_eq(html, "button", "onclick", "alert('clicked')");
        assert_attribute_eq(html, "a", "href", "https://loco.rs");

        let result = std::panic::catch_unwind(|| {
            assert_attribute_eq(html, "button", "onclick", "alert('wrong')");
        });

        assert!(result.is_err());
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Expected attribute 'onclick' with value 'alert('wrong')' for selector \
                  '\"button\"', but found a different value or no value."
            );
        }
    }

    #[test]
    fn test_assert_count() {
        let html = setup_test_html();
        assert_count(html, "ul#posts li", 3);

        let result = std::panic::catch_unwind(|| {
            assert_count(html, "ul#posts li", 1);
        });

        assert!(result.is_err());
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"Expected 1 elements matching selector '\"ul#posts li\"', but found 3 elements."
            );
        }
    }

    #[test]
    fn test_assert_css_eq_list() {
        let html = setup_test_html();
        assert_css_eq_list(html, "ul#posts li", &["Post 1", "Post 2", "Post 3"]);

        let result = std::panic::catch_unwind(|| {
            assert_css_eq_list(html, "ul#posts li", &["Post 1", "Post 2", "Wrong Post"]);
        });

        assert!(result.is_err());
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"assertion `left == right` failed: Expected texts [\"Post 1\", \"Post 2\", \
                  \"Wrong Post\"], but found [\"Post 1\", \"Post 2\", \"Post 3\"].\n  left: \
                  [\"Post 1\", \"Post 2\", \"Post 3\"]\n right: [\"Post 1\", \"Post 2\", \"Wrong \
                  Post\"]"
            );
        }
    }

    #[test]
    fn test_assert_css_eq_list_table() {
        let html = setup_test_html();
        assert_css_eq_list(
            html,
            "table tr td",
            &[
                "Post 1", "Author 1", "Post 2", "Author 2", "Post 3", "Author 3",
            ],
        );

        let result = std::panic::catch_unwind(|| {
            assert_css_eq_list(html, "table#posts_t tr td", &["Post 1", "Post 2", "Post 3"]);
        });

        assert!(result.is_err());
        if let Err(panic_message) = result {
            let panic_message = panic_message.downcast_ref::<String>().unwrap();
            assert_eq!(
                panic_message,
                &"assertion `left == right` failed: Expected texts [\"Post 1\", \"Post 2\", \
                  \"Post 3\"], but found [].\n  left: []\n right: [\"Post 1\", \"Post 2\", \"Post \
                  3\"]"
            );
        }
    }

    #[test]
    fn test_select() {
        let html = setup_test_html();
        assert_eq!(
            select(html, ".some-class"),
            vec!["<div class=\"some-class\">Some content here</div>"]
        );
        assert_eq!(
            select(html, "ul"),
            vec![
                "<ul id=\"posts\">\n                <li>Post 1</li>\n                <li>Post \
                 2</li>\n                <li>Post 3</li>\n            </ul>"
            ]
        );
    }
}
