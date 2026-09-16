Use AI to Convert Tera v1 Templates to Tera v2
==============================================

This document highlights the key considerations when migrating from Tera v1 to Tera v2 using AI tools.

A strong AI should get you at least 90% of the way there without any fuss, especially if you do not use Tera v1 macros.

However, the devil is in the details.  Subtle issues exist such that a converted termplate may
compile and run successfully, but still contain errors that may or may not surface during runtime.


Null Handling
-------------

In Tera v1, you can do `<p>{{ text }}</p>` to render a blank tag if `text` does not exist.

In Tera v2, this becomes an error -- the worst kind of error, one that crashes only at runtime.

The solution is to convert all nullable variable access to `expr or ""` which renders fine.

AI is good in doing this modification and is usually quite thorough.


Detecting Types
---------------

In Tera v2, there are now `is array`, `is string` etc.

You no longer have to use `is iterable` and not knowing whether a value is a string or an array etc.

AI should be good in doing this modification.


Patterns Need `pat=`
--------------------

In Tera v2, `is containing` requires `pat=` for the pattern.

Tera v1: `is containing("hello")`

Tera v2: `is containing(pat="hello")`

AI is very good in catching these.


Filters and Maps
----------------

In Tera v2, you can do `[item.key for item in collection if item.value != ""]`.

So Tera v1's `map` and `filter` filters are no longer needed.

AI can rewrite these expressions.


Default Filter
--------------

In Tera v2, `result | default(value=42)` is no longer necessary.

Replace with `result or 42` which is much cleaner and detects if `result` is truthy or falsy.

AI is good in doing this modification.

There is one catch: if `result` is falsy, it will be replaced with the default after the `or`.

`result or ""` will be blank if `result == 0`, so beware of this behavior difference between Tera v1 and v2.


Missing Filters
---------------

Many built-in filters from Tera v1 no longer load by default.

Some are no longer necessary, e.g. the `json_encode` filter is not needed as maps automatically render as JSON.

The old filters are in the `tera-contrib` crate.  This crate must be added to `Cargo.toml` to use them.


Coalescing Operators
--------------------

Tera v2 has coalescing operators such as `foo?.bar?.baz` and `foo?[42]` etc.

Use them with `or`: `foo?.bar?.baz or "N/A"`.

You no longer need: `{% if not foo or not foo.bar or not foo.bar.baz %}N/A{% else %}{{ foo.bar.baz }}{% endif %}`

AI is quite good in replace these expressions to use the coalescing operators if given clear instructions.


String Comparisons
------------------

Tera v2 now has built-in string comparisons, such as `>=`, `<` etc.

Your own custom filters in Tera v1 are no longer necessary.

If you have them, ask AI to convert them into the standard operators, which it'll do just fine.


Macros -> Components
--------------------

It is surprisingly easy to use AI to convert Tera v1 macros to Tera v2 components.

Simply ask AI to do that, and it'd successfully convert most of them automatically.

Ask the AI to eliminate all `import` statements which are also no longer needed.


Use Components as Values
------------------------

It is OK to use a Tera v2 component as value to a variable, just like Tera v1 macros.

Example: `{% set text = <foo bar={42} title="hello" /> %}`


Component Gotcha's
------------------

### Naming

Previously, macros are scoped to the individual template file.

In Tera v2, components are globally-scoped.

Therefore, you can now use one text name to rendre a component, instead of the file:macro
pair you needed in Tera v1.  The template file name no longe needs to be kept.

Suggested naming is to namespace the components, such as `foo.bar.baz`.

AI can easily do the renaming, or it can be done via a global search-and-replace.


### Varargs

Tera v2 components, different from Tera v1 macros, accept a _fixed_ number of arguments.

Extra, unrecognized arguments will cause an error.

The solution is to put `...rest` at the end of the arguments list, and Tera v2 will put
all unrecognized arguments into that variable (commonly named `rest`) which is a map.

AI is good at doing this.  You can also ask AI to only put `...rest` in components
where it detects additional arguments.


### Argument types

Tera v2 component arguments can be typed, and an argument with a value of the wrong type
will cause an error.

If a type is not specified, it is essentially `any`, which can be anything (except `null`).

If a type is specified, e.g. `option: map`, then it expects an argument of that type.

There is no _union_ typing, so it is either `any` (type omitted) or a fixed type.

Notice that a typed argument is _mandatory_.

AI is mostly OK in putting in these types.


### Default arguments

Tera v2 optional component arguments can be given defaults.

However, the gotcha is that they are also _typed_.

Therefore, an argument `option = true` will type the argument `option` to `bool`, and it
will no longer accept anything other than a `bool`.

There is no way to specify a default for an optional argument without binding its type.


### Rending components

You render a Tera v2 component this way: `<component arg1=... arg2=... />`.

However, a gotcha is that all argument values, other than strings, _MUST BE WRAPPED_ in `{...}`.

For example: `<component arg1="hello" arg2={42} arg3={ {"a":1, "b":2} } arg4={"hello" | caps} />`

Notice that `arg1`'s value does not need to be wrapped in `{...}` because it is a string.

`arg2` is not a string, so it must be wrapped in `{...}`.

`arg3` is a map, and you must put _SPACES_ between the braces, otherwise Tera confuses them to
be `{{ ... }}` interpolations.  So `{ { map } }` for maps.

`arg4` looks like a string, but it is actually an expression with a filter, so it must also be wrapped.

AI is good at doing the right thing, if you specify these rules clearly.  It might be best to start off
by wrapping everything, including strings, e.g. `arg={"hello"}`.  Then as the AI to do a second pass
to simplify wrapped literal strings.

Also, my AI occaionally screws up the `/>` parts, so beware.
