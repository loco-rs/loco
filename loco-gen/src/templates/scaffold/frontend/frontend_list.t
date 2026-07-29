to: frontend/src/pages/{{ snake_plural }}/List.tsx
skip_exists: true
message: "Frontend `List` page for `{{ pascal_singular }}` was added successfully."
injections:
- into: frontend/src/routes.tsx
  after: "// scaffold:imports"
  content: "{{ frontend_imports_injection }}"
- into: frontend/src/routes.tsx
  after: "// scaffold:routes"
  content: "{{ frontend_routes_injection }}"
---
import { Link } from "react-router";
import { useList{{ pascal_plural }}, useRemove{{ pascal_singular }} } from "../../api/{{ snake_plural }}";

export function List() {
  const { data, isLoading, isError, error } = useList{{ pascal_plural }}();
  const remove{{ pascal_singular }} = useRemove{{ pascal_singular }}();

  if (isLoading) {
    return <p>Loading…</p>;
  }

  if (isError) {
    return <p role="alert">{error.message}</p>;
  }

  return (
    <div>
      <h1>{{ pascal_plural }}</h1>
      <p>
        <Link to="/{{ snake_plural }}/new">New {{ pascal_singular }}</Link>
      </p>
      <table>
        <thead>
          <tr>
{%- for f in fields %}
            <th>{{ f.label }}</th>
{%- endfor %}
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {data?.items.map(({{ camel_singular }}) => (
            <tr key={{ "{" }}{{ camel_singular }}.id}>
{%- for f in fields %}
{%- if loop.first %}
              <td>
                <Link to={`/{{ snake_plural }}/${{ "{" }}{{ camel_singular }}.id}`}>{{ "{" }}{{ camel_singular }}.{{ f.field_name }}}</Link>
              </td>
{%- else %}
              <td>{{ "{" }}{{ camel_singular }}.{{ f.field_name }}}</td>
{%- endif %}
{%- endfor %}
              <td>
                <Link to={`/{{ snake_plural }}/${{ "{" }}{{ camel_singular }}.id}/edit`}>Edit</Link>{" "}
                <button
                  type="button"
                  disabled={remove{{ pascal_singular }}.isPending}
                  onClick={() => remove{{ pascal_singular }}.mutate({{ camel_singular }}.id)}
                >
                  Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {remove{{ pascal_singular }}.isError && <p role="alert">{remove{{ pascal_singular }}.error.message}</p>}
    </div>
  );
}
