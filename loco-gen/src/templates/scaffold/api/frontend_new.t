to: frontend/src/pages/{{ snake_plural }}/New.tsx
skip_exists: true
message: "Frontend `New` page for `{{ pascal_singular }}` was added successfully."
---
import { useState } from "react";
import { useNavigate } from "react-router";
import { useCreate{{ pascal_singular }} } from "../../api/{{ snake_plural }}";
import type { Create{{ pascal_singular }} } from "../../bindings/Create{{ pascal_singular }}";
{%- for e in enums %}
import type { {{ e.enum_type }} } from "../../bindings/{{ e.enum_type }}";
{%- endfor %}
{% for e in enums %}
const {{ e.options_const }}: {{ e.enum_type }}[] = [{% for v in e.variants %}"{{ v.value }}"{% if not loop.last %}, {% endif %}{% endfor %}];
{% endfor %}
export function New() {
  const navigate = useNavigate();
  const create{{ pascal_singular }} = useCreate{{ pascal_singular }}();

  const [form, setForm] = useState<Create{{ pascal_singular }}>({
{%- for f in fields %}
    {{ f.field_name }}: {{ f.initial_value }},
{%- endfor %}
  });

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    create{{ pascal_singular }}.mutate(form, {
      onSuccess: (created) => {
        navigate(`/{{ snake_plural }}/${created.id}`);
      },
    });
  }

  return (
    <div>
      <h1>New {{ pascal_singular }}</h1>
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
        <button type="submit" disabled={create{{ pascal_singular }}.isPending}>
          {create{{ pascal_singular }}.isPending ? "Creating…" : "Create"}
        </button>
        {create{{ pascal_singular }}.error && <p role="alert">{create{{ pascal_singular }}.error.message}</p>}
      </form>
    </div>
  );
}
