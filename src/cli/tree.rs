//! Terminal tree rendering for `cargo loco routes` and the generator's
//! "files created" summary. Pure formatting: no dispatch, no app state.

use std::{collections::BTreeMap, fmt::Write, path::PathBuf};

use colored::Colorize;

use crate::{
    app::{AppContext, Hooks},
    boot::list_endpoints,
};

#[derive(Default)]
struct RouteNode {
    children: BTreeMap<String, Self>,
    endpoints: Vec<(String, String)>,
}

impl RouteNode {
    fn is_leaf(&self) -> bool {
        self.endpoints.len() == 1 && self.children.is_empty()
    }

    fn is_collapsible(&self) -> bool {
        self.endpoints.is_empty()
            && self.children.len() == 1
            && self.children.values().next().is_some_and(Self::is_leaf)
    }

    fn method(&self) -> &str {
        self.endpoints
            .first()
            .map_or("", |(method, _)| method.as_str())
    }

    fn print(&self, prefix: &str, segment: &str, is_last: bool, is_root: bool, current_path: &str) {
        match (is_root, self.is_leaf(), self.is_collapsible()) {
            (true, true, _) => {
                Self::print_with_format(
                    &format!("/{segment}"),
                    &color_method(self.method()),
                    &Self::build_path(&[current_path, segment]),
                );
            }
            (true, _, true) => {
                let Some((child_segment, child_node)) = self.children.iter().next() else {
                    return;
                };
                Self::print_with_format(
                    &format!("/{segment}/{child_segment}"),
                    &color_method(child_node.method()),
                    &Self::build_path(&[current_path, segment, child_segment]),
                );
            }

            (false, true, _) => {
                let prefix_str = Self::format_prefix(prefix, is_last, true);

                Self::print_with_format(
                    &format!("{prefix_str}{segment}"),
                    &color_method(self.method()),
                    &Self::build_path(&[current_path, segment]),
                );
            }
            (false, _, true) => {
                let prefix_str = Self::format_prefix(prefix, is_last, true);
                let Some((child_segment, child_node)) = self.children.iter().next() else {
                    return;
                };
                Self::print_with_format(
                    &format!("{prefix_str}{segment}/{child_segment}"),
                    &color_method(child_node.method()),
                    &Self::build_path(&[current_path, segment, child_segment]),
                );
            }

            _ => {
                if is_root {
                    println!("/{segment}");
                } else if !segment.is_empty() {
                    println!("{}{}", Self::format_prefix(prefix, is_last, true), segment);
                }

                let next_prefix = Self::format_next_prefix(prefix, is_last);
                self.print_endpoints(
                    &next_prefix,
                    self.children.is_empty(),
                    &Self::build_path(&[current_path, segment]),
                );
                self.print_children(&next_prefix, &Self::build_path(&[current_path, segment]));
            }
        }
    }

    fn print_endpoints(&self, prefix: &str, is_last_group: bool, current_path: &str) {
        for (i, (method, _)) in self.endpoints.iter().enumerate() {
            let is_last_entry = i == self.endpoints.len() - 1 && is_last_group;
            let marker = if is_last_entry { "└─" } else { "├─" };
            Self::print_with_format(
                &format!("{prefix}{marker}"),
                &color_method(method),
                current_path,
            );
        }
    }

    fn print_children(&self, prefix: &str, current_path: &str) {
        let children = self.children.iter().collect::<Vec<_>>();
        for (i, (child_segment, child_node)) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;

            if child_node.is_leaf() {
                let marker = if is_last_child { "└─" } else { "├─" };
                Self::print_with_format(
                    &format!("{prefix}{marker} /{child_segment}"),
                    &color_method(child_node.method()),
                    &Self::build_path(&[current_path, child_segment]),
                );
            } else {
                child_node.print(prefix, child_segment, is_last_child, false, current_path);
            }
        }
    }

    fn format_prefix(prefix: &str, is_last: bool, with_slash: bool) -> String {
        let marker = if is_last { "└─" } else { "├─" };
        if with_slash {
            format!("{prefix}{marker} /")
        } else {
            format!("{prefix}{marker} ")
        }
    }

    fn format_next_prefix(prefix: &str, is_last: bool) -> String {
        if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        }
    }

    fn build_path(segments: &[&str]) -> String {
        segments.iter().fold(String::new(), |mut acc, &segment| {
            if !segment.is_empty() {
                acc.push('/');
                acc.push_str(segment);
            }
            acc.replace("//", "/")
        })
    }

    fn print_with_format(tree: &str, method: &str, full_path: &str) {
        println!("{:<50} {}", format!("{tree} {method}"), full_path);
    }
}

pub fn show_list_endpoints<H: Hooks>(ctx: &AppContext) {
    let mut routes = list_endpoints::<H>(ctx);
    routes.sort_by(|a, b| {
        let method_priority = |actions: &[_]| match actions
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
            .as_str()
        {
            "GET" => 0,
            "POST" => 1,
            "PUT" => 2,
            "PATCH" => 3,
            "DELETE" => 4,
            _ => 5,
        };
        a.uri
            .cmp(&b.uri)
            .then(method_priority(&a.actions).cmp(&method_priority(&b.actions)))
    });

    let mut route_tree = RouteNode::default();
    for router in routes {
        let path = router.uri.trim_start_matches('/');
        let segments: Vec<&str> = path.split('/').collect();
        if segments.is_empty() {
            continue;
        }

        let mut current_node = &mut route_tree;
        for segment in &segments {
            current_node = current_node
                .children
                .entry((*segment).to_string())
                .or_default();
        }

        current_node.endpoints.push((
            router
                .actions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            router.uri.clone(),
        ));
    }

    for (i, (segment, node)) in route_tree.children.iter().enumerate() {
        node.print("", segment, i == route_tree.children.len() - 1, true, "");
    }
}

fn color_method(method: &str) -> String {
    match method {
        "GET" => method.green().to_string(),
        "POST" => method.blue().to_string(),
        "PUT" => method.yellow().to_string(),
        "PATCH" => method.magenta().to_string(),
        "DELETE" => method.red().to_string(),
        _ => method.to_string(),
    }
}

#[must_use]
pub fn format_templates_as_tree(paths: Vec<PathBuf>) -> String {
    let mut categories: BTreeMap<String, BTreeMap<String, Vec<PathBuf>>> = BTreeMap::new();

    for path in paths {
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            let mut components = parent_str.split('/');
            if let Some(top_level) = components.next() {
                let top_key = top_level.to_string();
                let sub_key = components.next().unwrap_or("").to_string();

                categories
                    .entry(top_key)
                    .or_default()
                    .entry(sub_key)
                    .or_default()
                    .push(path);
            }
        }
    }

    let mut output = "Available templates and directories to copy:".to_string();
    let _ = writeln!(output);
    let _ = writeln!(output);

    for (top_level, sub_categories) in &categories {
        let _ = writeln!(output, "{}", top_level.clone().yellow());

        for (sub_category, paths) in sub_categories {
            if !sub_category.is_empty() {
                let _ = writeln!(output, "{}", format!(" └── {sub_category}").yellow());
            }

            for path in paths {
                let _ = writeln!(
                    output,
                    "   └── {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        }
    }

    let _ = writeln!(output);
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Usage Examples:".bold().green());
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Override a Specific File:".bold());

    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "scaffold/api/controller.t".yellow()
    );
    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "migration/add_columns.t".yellow()
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Override All Files in a Folder:".bold());
    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "scaffold/api".yellow()
    );

    let _ = writeln!(
        output,
        " * cargo loco generate override {}",
        "task".yellow()
    );
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", "Override All templates:".bold());
    let _ = writeln!(output, " * cargo loco generate override {}", ".".yellow());

    output
}
