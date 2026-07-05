to: frontend/src/pages/{{ snake_plural }}/Edit.tsx
skip_exists: true
message: "Frontend `Edit` page for `{{ pascal_singular }}` was added successfully."
---
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { use{{ pascal_singular }}, useUpdate{{ pascal_singular }} } from "../../api/{{ snake_plural }}";
{%- for e in enums %}
import type { {{ e.enum_type }} } from "../../bindings/{{ e.enum_type }}";
{%- endfor %}
import type { Update{{ pascal_singular }} } from "../../bindings/Update{{ pascal_singular }}";
{% for e in enums %}
const {{ e.options_const }}: {{ e.enum_type }}[] = [{% for v in e.variants %}"{{ v.value }}"{% if not loop.last %}, {% endif %}{% endfor %}];
{% endfor %}
export function Edit() {
  const navigate = useNavigate();
  const { id: idParam } = useParams<{ id: string }>();
  const id = Number(idParam);
  const isValidId = Number.isFinite(id) && id > 0;

  const { data, isLoading, isError, error } = use{{ pascal_singular }}(isValidId ? id : NaN);
  const update{{ pascal_singular }} = useUpdate{{ pascal_singular }}();

  const [form, setForm] = useState<Update{{ pascal_singular }} | null>(null);

  useEffect(() => {
    if (data) {
      setForm({
{%- for f in fields %}
        {{ f.field_name }}: data.{{ f.field_name }},
{%- endfor %}
      });
    }
  }, [data]);

  if (!isValidId) {
    return <p role="alert">Invalid {{ snake_singular }} id.</p>;
  }

  if (isLoading || !form) {
    return <p>Loading…</p>;
  }

  if (isError) {
    return <p role="alert">{error.message}</p>;
  }

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!form) {
      return;
    }
    update{{ pascal_singular }}.mutate(
      { id, data: form },
      {
        onSuccess: () => {
          navigate(`/{{ snake_plural }}/${id}`);
        },
      },
    );
  }

  return (
    <div>
      <h1>Edit {{ pascal_singular }}</h1>
      <form onSubmit={handleSubmit}>
{%- for f in fields %}
        <div>
          <label htmlFor="{{ f.field_name }}">{{ f.label }}</label>
{%- if f.input_kind == "select" %}
          <select
            id="{{ f.field_name }}"
            value={form.{{ f.field_name }}}
            onChange={(e) =>
              setForm({ ...form, {{ f.field_name }}: e.target.value as {{ f.enum_type }} })
            }
          >
            {{ "{" }}{{ f.options_const }}.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
{%- elif f.input_kind == "checkbox" %}
          <input
            id="{{ f.field_name }}"
            type="checkbox"
            checked={form.{{ f.field_name }}}
            onChange={(e) =>
              setForm({ ...form, {{ f.field_name }}: e.target.checked })
            }
          />
{%- elif f.input_kind == "date" %}
          <input
            id="{{ f.field_name }}"
            type="date"
            value={form.{{ f.field_name }} ?? ""}
            onChange={(e) =>
              setForm({
                ...form,
                {{ f.field_name }}: {% if f.nullable %}e.target.value || null{% else %}e.target.value{% endif %},
              })
            }
{%- if not f.nullable %}
            required
{%- endif %}
          />
{%- elif f.input_kind == "datetime" %}
          <input
            id="{{ f.field_name }}"
            type="datetime-local"
            value={form.{{ f.field_name }} ?? ""}
            onChange={(e) =>
              setForm({
                ...form,
                {{ f.field_name }}: {% if f.nullable %}e.target.value || null{% else %}e.target.value{% endif %},
              })
            }
{%- if not f.nullable %}
            required
{%- endif %}
          />
{%- elif f.input_kind == "number" %}
          <input
            id="{{ f.field_name }}"
            type="number"
            value={form.{{ f.field_name }}}
            onChange={(e) =>
              setForm({ ...form, {{ f.field_name }}: Number(e.target.value) })
            }
{%- if not f.nullable %}
            required
{%- endif %}
          />
{%- elif f.input_kind == "text_number" %}
          <input
            id="{{ f.field_name }}"
            type="text"
            value={form.{{ f.field_name }}}
            onChange={(e) => setForm({ ...form, {{ f.field_name }}: e.target.value })}
{%- if not f.nullable %}
            required
{%- endif %}
          />
{%- elif f.input_kind == "textarea" %}
          <textarea
            id="{{ f.field_name }}"
            value={form.{{ f.field_name }}}
            onChange={(e) => setForm({ ...form, {{ f.field_name }}: e.target.value })}
{%- if not f.nullable %}
            required
{%- endif %}
          />
{%- else %}
          <input
            id="{{ f.field_name }}"
            type="text"
            value={form.{{ f.field_name }}}
            onChange={(e) => setForm({ ...form, {{ f.field_name }}: e.target.value })}
{%- if not f.nullable %}
            required
{%- endif %}
          />
{%- endif %}
        </div>
{%- endfor %}
        <button type="submit" disabled={update{{ pascal_singular }}.isPending}>
          {update{{ pascal_singular }}.isPending ? "Saving…" : "Save"}
        </button>
        {update{{ pascal_singular }}.error && <p role="alert">{update{{ pascal_singular }}.error.message}</p>}
      </form>
    </div>
  );
}
