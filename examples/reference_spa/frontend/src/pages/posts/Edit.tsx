import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { usePost, useUpdatePost } from "../../api/posts";
import type { PostStatus } from "../../bindings/PostStatus";
import type { UpdatePost } from "../../bindings/UpdatePost";

const STATUS_OPTIONS: PostStatus[] = ["draft", "published", "archived"];

export function Edit() {
  const navigate = useNavigate();
  const { id: idParam } = useParams<{ id: string }>();
  const id = Number(idParam);
  const isValidId = Number.isFinite(id) && id > 0;

  const { data, isLoading, isError, error } = usePost(isValidId ? id : NaN);
  const updatePost = useUpdatePost();

  const [form, setForm] = useState<UpdatePost | null>(null);

  useEffect(() => {
    if (data) {
      setForm({
        title: data.title,
        content: data.content,
        status: data.status,
        price: data.price,
      });
    }
  }, [data]);

  if (!isValidId) {
    return <p role="alert">Invalid post id.</p>;
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
    updatePost.mutate(
      { id, data: form },
      {
        onSuccess: () => {
          navigate(`/posts/${id}`);
        },
      },
    );
  }

  return (
    <div>
      <h1>Edit Post</h1>
      <form onSubmit={handleSubmit}>
        <div>
          <label htmlFor="title">Title</label>
          <input
            id="title"
            type="text"
            value={form.title}
            onChange={(e) => setForm({ ...form, title: e.target.value })}
            required
          />
        </div>
        <div>
          <label htmlFor="content">Content</label>
          <textarea
            id="content"
            value={form.content}
            onChange={(e) => setForm({ ...form, content: e.target.value })}
            required
          />
        </div>
        <div>
          <label htmlFor="status">Status</label>
          <select
            id="status"
            value={form.status}
            onChange={(e) =>
              setForm({ ...form, status: e.target.value as PostStatus })
            }
          >
            {STATUS_OPTIONS.map((status) => (
              <option key={status} value={status}>
                {status}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label htmlFor="price">Price</label>
          <input
            id="price"
            type="text"
            value={form.price}
            onChange={(e) => setForm({ ...form, price: e.target.value })}
            required
          />
        </div>
        <button type="submit" disabled={updatePost.isPending}>
          {updatePost.isPending ? "Saving…" : "Save"}
        </button>
        {updatePost.error && <p role="alert">{updatePost.error.message}</p>}
      </form>
    </div>
  );
}
