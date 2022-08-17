# Span - Pandoc-based static site generator

Span is a [Pandoc](https://pandoc.org)-based static site generator.
It is a work in progress: a section marked with "(WIP)" below means that the features described in that section are not currently supported in Span. The documentation for WIP features details my current plan for how Span will behave.
Currently the only way to install Span is by cloning this repository and compiling it with `cargo`. Eventually that will change.

## folder layout

```
./
    content/
    snippets/
    templates/
```

The **content** of the website goes in `content/`; your paths are a direct reflection of the folder structure (e.g. `/content/blog/interesting.md ` becomes example.com/content/blog/interesting).

**Templates** are Pandoc templates used to render your content.

**Snippets** are files that you can use to de-duplicate your content. They're kind of like Pandoc's partials, but use a different syntax so that you can use Pandoc's partials in templates, where they're designed to be used.

## template and snippet selection logic

Snippets WIP.

When you specify a snippet or template, Span uses the following logic for figuring out which snippet/template to use.

Given the following site structure:

```
./
    content/
        index.md
        about.md
        blog/
            interesting.md
            awesome.md
            cool.md
        notes/
            contents.md
            2021/
                note.md
            2022/
                contents.md
                note1.md
                note2.md
    snippets/
        navbar.html
        blog/
            navbar.html
        notes/
            author.html
    templates/
        default.html
        blog/
            default.html
        notes/
            default.html
            contents.html
```

**Template logic**: Without any template specified in the YAML metadata, Span will use the closest matching template. Span considers files in the closest matching folder, looking in the parent only if necessary, and using a matching name (without extension), or `default.html` templates when possible. Consider `notes/2022/note1.md`. Span looks under templates, but can't find `templates/notes/2022/`, so it jumps to the parent, `templates/notes`, and looks there. This has a `default.html`, so Span will use this template to render `notes/2022/note1.md`. Below is a table of sample files and what template it will use:

| content file             | template file         |
|--------------------------|-----------------------|
| `index.md`               | `default.html`        |
| `blog/cool.md`           | `blog/default.html`   |
| `notes/contents.md`      | `notes/contents.html` |
| `notes/2022/contents.md` | `notes/contents.html` |

You can always override this logic by specifying a full path (starting with `templates/`) to the template you want to use.

**Snippet logic**: Snippet logic is very similar to template logic, and uses the name given to match against possible snippets. Using the example folder structure, files in `content/` use `snippets/navbar.html`, files in `content/blog/` use `snippets/blog/navbar.html`, but files in `content/notes/` use `snippets/navbar.html` because only the parent has a matching snippet.

You can always override this logic by specifying a full path (starting with `snippets/`) to the snippet you want to use.

## bare minimum layout

```
./
    content/
        index.md
    snippets/
    templates/
        default.html
```

You will need to populate `templates/default.html` with a default Pandoc template. A simple way to do this is by running `pandoc -D html > templates/default.html`.

## settings (WIP)

Currently there are no settings, but that will change soon.

---

Thanks for checking out `span`!

Ved Thiru (@PerpetualCreativity on GitHub, vulcan#0604 on Discord)

