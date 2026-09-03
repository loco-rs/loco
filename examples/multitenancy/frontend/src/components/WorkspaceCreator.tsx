import { useState, type FormEvent } from "react";
import { useCreateWorkspace } from "../api/auth";
import { tenantSlug } from "../auth/tenant";
import type { Workspace } from "../bindings/Workspace";

export function WorkspaceCreator({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (workspace: Workspace) => void;
}) {
  const createWorkspace = useCreateWorkspace();
  const [name, setName] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const slug = tenantSlug(name);
    if (!slug) {
      setValidationError("Use at least one letter or number in the workspace name.");
      return;
    }

    createWorkspace.mutate(
      { tenant_name: name.trim() },
      { onSuccess: onCreated },
    );
  }

  return (
    <div
      className="workspace-modal-backdrop"
      role="presentation"
      onMouseDown={onClose}
    >
      <section
        className="panel workspace-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="workspace-modal-title"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            onClose();
          }
        }}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          className="modal-close"
          type="button"
          aria-label="Close workspace form"
          onClick={onClose}
        >
          ×
        </button>
        <span className="eyebrow">New tenant</span>
        <h2 id="workspace-modal-title">Create a workspace</h2>
        <p className="form-description">
          You will become the workspace owner and can manage its members and
          core features and optional add-ons.
        </p>
        <form onSubmit={handleSubmit}>
          <label htmlFor="workspace-name">Workspace name</label>
          <input
            id="workspace-name"
            autoFocus
            minLength={2}
            maxLength={100}
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              setValidationError(null);
            }}
            placeholder="Research team"
            required
          />
          <p className="hint">
            The workspace slug is generated automatically and scoped by tenant ID.
          </p>
          {(validationError || createWorkspace.error) && (
            <p className="error" role="alert">
              {validationError ?? createWorkspace.error?.message}
            </p>
          )}
          <div className="modal-actions">
            <button className="secondary" type="button" onClick={onClose}>
              Cancel
            </button>
            <button
              className="primary"
              type="submit"
              disabled={createWorkspace.isPending}
            >
              {createWorkspace.isPending ? "Creating…" : "Create workspace"}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}
