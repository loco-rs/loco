to: frontend/src/pages/{{ snake_plural }}/Show.tsx
skip_exists: true
message: "Frontend `Show` page for `{{ pascal_singular }}` was added successfully."
---
import { Link, useParams } from "react-router";
import { use{{ pascal_singular }} } from "../../api/{{ snake_plural }}";

export function Show() {
  const { id: idParam } = useParams<{ id: string }>();
  const id = Number(idParam);
  const isValidId = Number.isFinite(id) && id > 0;

  const { data, isLoading, isError, error } = use{{ pascal_singular }}(isValidId ? id : NaN);

  if (!isValidId) {
    return <p role="alert">Invalid {{ snake_singular }} id.</p>;
  }

  if (isLoading) {
    return <p>Loading…</p>;
  }

  if (isError) {
    return <p role="alert">{error.message}</p>;
  }

  if (!data) {
    return <p>{{ pascal_singular }} not found.</p>;
  }

  return (
    <div>
      <h1>{data.{{ title_field_name }}}</h1>
      <dl>
{%- for f in fields %}
{%- if f.field_name != title_field_name %}
        <dt>{{ f.label }}</dt>
        <dd>{data.{{ f.field_name }}{% if f.nullable %} ?? "—"{% endif %}}</dd>
{%- endif %}
{%- endfor %}
{%- if with_tz %}
        <dt>Created at</dt>
        <dd>{data.created_at}</dd>
        <dt>Updated at</dt>
        <dd>{data.updated_at}</dd>
{%- endif %}
      </dl>
      <p>
        <Link to={`/{{ snake_plural }}/${data.id}/edit`}>Edit</Link>{" "}
        <Link to="/{{ snake_plural }}">Back to list</Link>
      </p>
    </div>
  );
}
