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

## snippet syntax

The syntax for snippets is similar to the Pandoc partial syntax. To use a snippet in a file use `$%%{snippet_name(val1: Hello, val2: World)}`. In order to iterate over the metadata of files in a folder in `contents/` use `$%%{path/to/folder:snippet_name(val1: hello, val2: world)}` (this snippet will be called once for every file that is an immediate child of the folder, with `data` passed in). The metadata is taken from the YAML metadata block at the top of the file (the same block that Pandoc uses). Example snippet below:

```
### $%{data.title}

*${data.date}*, $%{data.author}

$%{data.description}

$%{val1} | $%{val2}
```

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

## configuration

The configuration file is `./span.yml` (in your site folder). An example configuration is below.

```yaml
# '*' in files means "anything here"

# ignore (do not process and do not run) matching files
ignore:
  - "drafts/*" # this will ignore files in the contents/drafts folder

# do not process, only output matching files
# if a file is specified in pre-run, that process
# *is* run on the file and then that file is output
passthrough:
  - "raw/*" # this will pass-through files in the contents/raw folder

# process matching files using the specified commands
pre-run:
  # span will run these commands, substituting %i for path to input files
  # and %o for path to output files
  # if %i is not in the command, span will pass the contents of the input
  # file to stdin. if %o is not in the command string, span will use stdout
  # as the output
  - command: "vale"
    files:
      - "*.md"
      - "*.txt"
    error_on: "status" # default: stderr, possible values are stdout, stderr, status, none
    replace: false # default: true
  - command: "proselint"
    files:
      - "*.md"
    error_on: "stdout"
    replace: false
  - command: "./tailwindcss -i %i -o %o --minify"
    files:
      - "main.css"
    error_on: "none"
    replace: true

# pandoc filters to run on matching files
filters:
  - path: "~/scripts/pandoc-asciimath2tex"
    files:
      - "notes/math/*"
      - "notes/physics/*"

# extra args to pass to pandoc
# span automatically passes --to=html5, --standalone, and --template=
# (and --filter if necessary)
extra_args:
  - "--katex"

# default template name to use
default_template: default.html # default: default.html
```

---

Thanks for checking out `span`!

Ved Thiru (@PerpetualCreativity on GitHub, vulcan#0604 on Discord)

