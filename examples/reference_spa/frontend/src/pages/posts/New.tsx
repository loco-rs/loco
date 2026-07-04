import { useState } from "react";
import { useNavigate } from "react-router";
import { useCreatePost } from "../../api/posts";
import type { CreatePost } from "../../bindings/CreatePost";
import type { PostStatus } from "../../bindings/PostStatus";

const STATUS_OPTIONS: PostStatus[] = ["draft", "published", "archived"];

export function New() {
  const navigate = useNavigate();
  const createPost = useCreatePost();

  const [form, setForm] = useState<CreatePost>({
    title: "",
    content: "",
    status: "draft",
    price: "",
  });

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createPost.mutate(form, {
      onSuccess: (created) => {
        navigate(`/posts/${created.id}`);
      },
    });
  }

  return (
    <div>
      <h1>New Post</h1>
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
        <button type="submit" disabled={createPost.isPending}>
          {createPost.isPending ? "Creating…" : "Create"}
        </button>
        {createPost.error && <p role="alert">{createPost.error.message}</p>}
      </form>
    </div>
  );
}
