export interface Slide {
  k: string;
  title: string;
  bullets: string[];
}

export interface CodeEntry {
  file: string;
  html: string;
}

export const SLIDES: Slide[] = [
  {k:'controller', title:'Handle the request.', bullets:[
    '<b>No macros, no magic.</b> Plain <code>async fn</code> returning <code>Result&lt;Response&gt;</code>.',
    '<b>Wired with Routes.</b> The app context — db, config, workers — is handed to you.',
    '<b>Typed both ways.</b> Extractors in, serialized responses out.']},
  {k:'model', title:'Your data and its logic.', bullets:[
    '<b>Entities plus your methods.</b> Queries and rules live next to the data.',
    '<b>No boilerplate.</b> No stringly-typed SQL, no repository layer.',
    '<b>Typed end to end.</b> The compiler checks your columns and relations.']},
  {k:'worker', title:'Move slow work off the request path.', bullets:[
    '<b>Typed arguments.</b> Jobs run out of band, not on the request.',
    '<b>Enqueue with <code>perform_later</code>.</b> Fire and forget from a controller.',
    '<b>Any queue.</b> Postgres, SQLite, or Redis — same code, no rewrite.']},
  {k:'view', title:'Shape what leaves the API.', bullets:[
    '<b>Decide the wire format.</b> Serializers choose exactly what ships.',
    '<b>Decoupled from the schema.</b> Your API shape is not your table shape.',
    '<b>Evolve independently.</b> Change one without breaking the other.']},
  {k:'task', title:'Run ops from the CLI, or on a schedule.', bullets:[
    '<b>Typed commands.</b> Run with <code>cargo loco task</code> — backfills, seeds, maintenance.',
    '<b>On a schedule.</b> The built-in scheduler runs them on cron.',
    '<b>Part of the app.</b> Registered and typed — no stray scripts.']},
  {k:'mailer', title:'Transactional email, templated.', bullets:[
    '<b>Templated.</b> Subject, text, and HTML rendered from templates.',
    '<b>Sends over SMTP.</b> A dev mailer catches messages locally.',
    '<b>In the box.</b> No third-party email service to wire up.']}
];

export const CODE: Record<string, CodeEntry> = {
  controller:{file:'src/controllers/articles.rs', html:`<span class="c">// src/controllers/articles.rs</span>
<span class="k">use</span> loco_rs::prelude::*;
<span class="k">use</span> <span class="k">crate</span>::models::_entities::articles;
<span class="k">use</span> <span class="k">crate</span>::views::article::ArticleResponse;

<span class="k">pub async fn</span> <span class="fn">list</span>(State(ctx): State&lt;AppContext&gt;) -&gt; <span class="t">Result</span>&lt;Response&gt; {
    <span class="k">let</span> items = articles::Model::latest(&amp;ctx.db).<span class="k">await</span>?;
    <span class="k">let</span> res: Vec&lt;_&gt; = items.iter().map(ArticleResponse::new).collect();
    format::json(res)
}

<span class="k">pub fn</span> <span class="fn">routes</span>() -&gt; <span class="t">Routes</span> {
    <span class="t">Routes</span>::new()
        .prefix(<span class="s">"articles"</span>)
        .add(<span class="s">"/"</span>, get(list))
}`},
  model:{file:'src/models/articles.rs', html:`<span class="c">// src/models/articles.rs</span>
<span class="k">use</span> loco_rs::prelude::*;
<span class="k">use</span> <span class="k">super</span>::_entities::articles::{<span class="k">self</span>, ActiveModel, Entity, Model};

<span class="k">impl</span> <span class="t">ActiveModelBehavior</span> <span class="k">for</span> <span class="t">ActiveModel</span> {}

<span class="k">impl</span> <span class="t">Model</span> {
    <span class="c">/// The ten most recent articles, newest first.</span>
    <span class="k">pub async fn</span> <span class="fn">latest</span>(db: &amp;DatabaseConnection) -&gt; <span class="t">Result</span>&lt;Vec&lt;Model&gt;&gt; {
        <span class="t">Entity</span>::find()
            .order_by_desc(articles::Column::CreatedAt)
            .limit(<span class="n">10</span>)
            .all(db)
            .<span class="k">await</span>
            .map_err(Into::into)
    }
}`},
  worker:{file:'src/workers/digest.rs', html:`<span class="c">// src/workers/digest.rs</span>
<span class="k">use</span> loco_rs::prelude::*;
<span class="k">use</span> serde::{Deserialize, Serialize};

<span class="k">pub struct</span> <span class="t">DigestWorker</span> { <span class="k">pub</span> ctx: AppContext }

<span class="c">#[derive(Debug, Serialize, Deserialize)]</span>
<span class="k">pub struct</span> <span class="t">DigestArgs</span> { <span class="k">pub</span> user_id: <span class="t">i64</span> }

<span class="c">#[async_trait]</span>
<span class="k">impl</span> <span class="t">BackgroundWorker</span>&lt;DigestArgs&gt; <span class="k">for</span> <span class="t">DigestWorker</span> {
    <span class="k">fn</span> <span class="fn">build</span>(ctx: &amp;AppContext) -&gt; <span class="t">Self</span> { <span class="t">Self</span> { ctx: ctx.clone() } }

    <span class="k">async fn</span> <span class="fn">perform</span>(&amp;<span class="k">self</span>, args: DigestArgs) -&gt; <span class="t">Result</span>&lt;()&gt; {
        <span class="c">// runs off the request path</span>
        <span class="t">Mailer</span>::daily_digest(&amp;<span class="k">self</span>.ctx, args.user_id).<span class="k">await</span>?;
        <span class="t">Ok</span>(())
    }
}`},
  view:{file:'src/views/article.rs', html:`<span class="c">// src/views/article.rs</span>
<span class="k">use</span> serde::Serialize;
<span class="k">use</span> <span class="k">crate</span>::models::_entities::articles;

<span class="c">#[derive(Debug, Serialize)]</span>
<span class="k">pub struct</span> <span class="t">ArticleResponse</span> {
    <span class="k">pub</span> id: <span class="t">i64</span>,
    <span class="k">pub</span> title: <span class="t">String</span>,
    <span class="k">pub</span> excerpt: <span class="t">String</span>,
}

<span class="k">impl</span> <span class="t">ArticleResponse</span> {
    <span class="k">pub fn</span> <span class="fn">new</span>(a: &amp;articles::Model) -&gt; <span class="t">Self</span> {
        <span class="t">Self</span> {
            id: a.id,
            title: a.title.clone(),
            excerpt: a.content.chars().take(<span class="n">140</span>).collect(),
        }
    }
}`},
  task:{file:'src/tasks/seed_demo.rs', html:`<span class="c">// src/tasks/seed_demo.rs</span>
<span class="k">use</span> loco_rs::prelude::*;
<span class="k">use</span> <span class="k">crate</span>::models::_entities::articles;

<span class="k">pub struct</span> <span class="t">SeedDemo</span>;

<span class="c">#[async_trait]</span>
<span class="k">impl</span> <span class="t">Task</span> <span class="k">for</span> <span class="t">SeedDemo</span> {
    <span class="k">fn</span> <span class="fn">task</span>(&amp;<span class="k">self</span>) -&gt; <span class="t">TaskInfo</span> {
        <span class="t">TaskInfo</span> { name: <span class="s">"seed_demo"</span>.into(), detail: <span class="s">"Seed demo articles"</span>.into() }
    }

    <span class="k">async fn</span> <span class="fn">run</span>(&amp;<span class="k">self</span>, ctx: &amp;AppContext, _vars: &amp;task::Vars) -&gt; <span class="t">Result</span>&lt;()&gt; {
        db::seed::&lt;articles::ActiveModel&gt;(&amp;ctx.db, <span class="s">"articles.yaml"</span>).<span class="k">await</span>?;
        <span class="t">Ok</span>(())
    }
}`},
  mailer:{file:'src/mailers/article.rs', html:`<span class="c">// src/mailers/article.rs</span>
<span class="k">use</span> loco_rs::prelude::*;
<span class="k">use</span> <span class="k">crate</span>::models::_entities::users;

<span class="k">static</span> welcome: Dir = <span class="fn">include_dir!</span>(<span class="s">"src/mailers/article/welcome"</span>);

<span class="k">pub struct</span> <span class="t">ArticleMailer</span> {}
<span class="k">impl</span> <span class="t">Mailer</span> <span class="k">for</span> <span class="t">ArticleMailer</span> {}

<span class="k">impl</span> <span class="t">ArticleMailer</span> {
    <span class="k">pub async fn</span> <span class="fn">welcome</span>(ctx: &amp;AppContext, user: &amp;users::Model) -&gt; <span class="t">Result</span>&lt;()&gt; {
        <span class="t">Self</span>::mail_template(ctx, &amp;welcome, mailer::Args {
            to: user.email.clone(),
            locals: json!({ <span class="s">"name"</span>: user.name }),
            ..<span class="t">Default</span>::default()
        }).<span class="k">await</span>?;
        <span class="t">Ok</span>(())
    }
}`}
};

export function stripTags(html: string): string {
  return html.replace(/<[^>]+>/g, '')
    .replace(/&lt;/g,'<').replace(/&gt;/g,'>').replace(/&amp;/g,'&');
}
