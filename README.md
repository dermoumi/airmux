# Rmux

Just another tool that allows you to configure tmux sessions using YAML (or JSON). Tmux 3.0+ is required.

- [Rmux](#rmux)
  - [Usage](#usage)
    - [Example Rmux project:](#example-rmux-project)
    - [Starting a session](#starting-a-session)
    - [Create and edit project files](#create-and-edit-project-files)
      - [Project definition](#project-definition)
      - [Commands](#commands)
      - [Window definition](#window-definition)
      - [Pane definition](#pane-definition)
      - [Layouts](#layouts)
      - [Environment variables and parameter expansion](#environment-variables-and-parameter-expansion)
    - [Local project files](#local-project-files)
    - [Other commands](#other-commands)
      - [List all projects](#list-all-projects)
      - [Stop the session corresponding to a project](#stop-the-session-corresponding-to-a-project)
      - [Delete a project](#delete-a-project)
      - [Debug session creation](#debug-session-creation)
      - [Save current session as a project](#save-current-session-as-a-project)
  - [Copyright](#copyright)

## Usage

```
USAGE:
    rmux [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config-dir <DIR>     configuration directory to use [env: RMUX_CONFIG=]

SUBCOMMANDS:
    debug     only print tmux command source without actually running tmux
    edit      Create or edit a project
    freeze    Save current tmux session as a project file (commands not included)
    help      Prints this message or the help of the given subcommand(s)
    kill      Kill tmux session that matches the project
    list      List all configured projects
    remove    Remove a project (does not affect loaded tmux sessions)
    start     Start a project as a tmux session
```

### Example Rmux project:

```yaml
# Set project working directory
working_dir: ~/projects/django_project/

# Run dev redis and postgresql instances when the session is created
on_create:
  - >- # yaml-multiline.info
    docker run -itd
    --name r-redis
    --publish 16379:6379
    redis || docker start r-redis
  - >-
    docker run -itd
    --name r-postgresql
    --publish 15432:5432
    --env POSTGRES_PASSWORD=hunter2
    --volume $HOME/.postgresql:/var/lib/postgresql/13
    postgresql:13 || docker start r-postgresql

# Stop dev redis and postgresql when the session is stopped
on_stop: docker stop r-redis r-postgresql

# Activate python virtualenv on each of this session's panes
pane_commands: source .venv/bin/activate

# Define tmux windows
windows:
  # First window contains 3 panes: an empty shell, the dev server output and the worker output
  - main:
    panes:
      -
      - split: v # Split previous pane by half, vertically
        command: python manage.py runserver
      - split: h # split previous pane by half, horizontally
        command: python celery --app django_project worker
  # Second window contains logs of the previously run docker containers
  - container-logs:
    panes:
      - docker logs -f r-postgresql
      - docker logs -f r-redis
```

### Starting a session

```console
$ rmux start my_project [param1 [param2...]]
```

Parameters are accessible in the project file as `$1`, `$2`, etc...

### Create and edit project files

Create or edit projects using:

```console
$ rmux edit <my_project>
```

The default editor (`$EDITOR`) is used to open the file.
You can use the `--editor` option to specify which editor to use:

```console
$ rmux edit --editor="code -w" my_project
```

#### Project definition

All the fields are optional, but at least one is required.

```yaml
# Name of the session in tmux. Cannot contain a dot (.) or colon (:) (alias: name)
session_name: <project name>

# Tmux command to use. Can also be overritten by using `--command` when running rmux
tmux_command: tmux

# Flags and options to pass to tmux every time it's executed
tmux_options: <empty>

# Socket name to pass to tmux (alias: socket_name)
# Equivalent to adding `-L <socket_name>` to `tmux_options`
tmux_socket: <empty>

# Working directory for all the windows in this session (alias: root)
# If declared and left empty or set to ~ (null in Json), defaults to $HOME instead.
working_dir: <current working directory>

# The starting index for windows (should be a non-negative integer)
window_base_index: 1

# The starting index for panes (should be a non-negative integer)
pane_base_index: 1

# Name of index window that's selected on startup
startup_window: <first window>

# Index of pane that's selected on startup
startup_pane: <first pane>

# Shell commands to execute before the session is attached (alias: on_project_start)
# Available substitutions: __TMUX__, __SESSION__
on_start:

# Shell commands to execute before the session is attached the first time (alias: on_project_first_start)
# Available substitutions: __TMUX__, __SESSION__
on_first_start:

# Shell commands to execute before the session is re-attached (alias: on_project_restart)
# Available substitutions: __TMUX__, __SESSION__
on_restart:

# Shell commands to execute after the session is detached (alias: on_project_exit)
# Available substitutions: __TMUX__
on_exit:

# Shell commands to execute after the session is destroyed (alias: on_project_stop)
# Available substitutions: __TMUX__
on_stop:

# Shell commands to execute after the session (with all its content) is created
# Available substitutions: __TMUX__, __SESSION__
post_create:

# Shell commands to execute when a pane is created (before pane_commands are typed in)
# Available substitutions: __TMUX__, __SESSION__, __WINDOW__, __PANE__
on_pane_create:

# Shell commands to execute after a pane is created (after pane_commands are typed in)
# Available substitutions: __TMUX__, __SESSION__, __WINDOW__, __PANE__
post_pane_create:

# Commands that are executed in the shell of each pane (alias: pre_window, pane_command)
pane_commands:

# Whether or not the session automatically attaches on creation (can't use both)
attach: true
detached: false

# Window declarations (alias: window)
windows: <default empty window>
```

#### Commands

Commands can be written either as a string or as a list of strings:

```yaml
# Run a single command when attaching the session
on_start: echo command1
# Run 3 commands when detaching the session
on_stop:
    - echo command2
    - echo command3
    - echo command4
```

Note: Rmux will always remove `\r` characters and replace `\n` with a space character (` `),
even if you don't use the correct yaml multiline syntax.

All commands are executed regardless of the exit status of the previous command.

#### Window definition

All the fields are optional, but at least one is required.

```yaml
windows:
  # Window names should not contain dots (.) and colons (:)
  # You can have multiple windows with the same name
  # It can also have a null (~) name, in which case it'll use default tmux behavior
  - window_1_name:
      # Working directory for the window (alias: root)
      # If declared and left empty or set to ~ (null in Json), defaults to $HOME instead.
      working_dir: <project's working directory>

      # Layout, can be: even-horizontal, even-vertical, main-horizontal, main-vertical, tiled
      # Or a custom layout, see `Layouts` section for details. Can't use with custom pane splits.
      layout: <no_layout>

      # Shell commands to execute when a window is created (before child pane panes are configured)
      # Available substitutions: __TMUX__, __SESSION__, __WINDOW__
      on_create:

      # Shell commands to execute after a window is created (after all child panes are configured)
      # Available substitutions: __TMUX__, __SESSION__, __WINDOW__
      post_create:

      # Shell commands to execute when a pane is created (before pane_commands are typed in)
      # Available substitutions: __TMUX__, __SESSION__, __WINDOW__, __PANE__
      on_pane_create:

      # Shell commands to execute after a pane is created (after pane_commands are typed in)
      # Available substitutions: __TMUX__, __SESSION__, __WINDOW__, __PANE__
      post_pane_create:

      # Commands that are executed in the shell of each pane (alias: pre, pane_command)
      pane_commands:

      # List of panes
      panes: <default empty pane>
```

Windows can also be defined as a single command or multiple commands (one for each pane):

```yaml
windows:
  - echo "single pane nameless window"
  - single_pane: "single pane named window"
  - ~:
    - echo "pane 1 of a nameless window"
    - echo "pane 2 of a nameless window"
  - multiple_panes:
    - echo "pane 1 of a named window"
    - echo "pane 2 of a named window"
```

Also, window definition fields can be at the same level as the name, as long as it is the first key:

```yaml:
windows:
  - window_name:
    layout: main-vertical
    panes:
      - echo cmd1
      - echo cmd2
```

You can also override the window name with the `name` (alias: `title`) field:

```yaml:
windows:
  - layout: main-vertical
    name: ~
    panes:
      - echo cmd1
      - echo cmd2
```

#### Pane definition

```yaml
panes:
  - # Working directory for the pane (alias: root)
    # If declared and left empty or set to ~ (null in Json), defaults to $HOME instead.
    working_dir: <window's working directory>

    # The pane to split from when creating this one. Does not apply to the first pane.
    # These indexes follow the same order as in the project file and always start with pane_base_index
    split_from: <previous pane>

    # How to split when creating this pane (v, vertical, h, horizontal)
    split: horizontal

    # Size of this pane (number of columns/rows or a percentage)
    split_size: 50%

    # Whether or not to send a clear combination (Ctrl+L) after typing the pane commands
    clear: false

    # Shell commands to execute when a pane is created (before pane_commands are typed in)
    # Available substitutions: __TMUX__, __SESSION__, __WINDOW__
    on_create:

    # Shell commands to execute after a pane is created (before pane_commands are typed in)
    # Available substitutions: __TMUX__, __SESSION__, __WINDOW__
    post_create:

    # Commands to type and run in this pane's shell (alias: command)
    commands:
```

#### Layouts

Aside from the 5 default layouts, you can also supply a custom layout:

```yaml
windows:
  - layout: 'fc16,277x30,0,0{137x30,0,0,2,139x30,138,0,3}'
```

You can read more about [custom layouts on the Tao of tmux][custom_layouts_tao]

When using a layout on a window, all `split` and `split_size` cannot be used on the underlying panes.

[custom_layouts_tao]: https://leanpub.com/the-tao-of-tmux/read#window-layouts

#### Environment variables and parameter expansion

The project file supports expansion of variables (anywhere, not just string values)

```yaml
windows:
  # Expanding the User environment variable
  - $USER:
  # Alternative syntax, allows you to use mid-words
  - ${EDITOR}_window:
  # Expansion with a fallback value for when the variable is not set
  - ${SOME_VAR:-fallback_value}:
```

This also means that whenever you need to write `$` you'll need to escape as `$$`.

Furthermore, any extra values passed to `rmux start` or `rmux kill` are available as `$1`, `$2`, etc...

```yaml
windows:
  - pipenv run server ${1:-8000}
```

```bash
# pipenv run server will run at port 8080 instead of the default 8000
$ rmux start my_project 8080
```

### Local project files

Commands that accept a project name can be called without it to use a local `.rmux.(yml|yaml|json)` project file
instead.

If no local project file exists, Rmux will look into each ancestor to the current working directory until it finds one.
Otherwise, it will default to `.rmux.yml` on the current directory.

You can specify the extension of the local project file when creating it:

```console
$ rmux edit --ext json
```

### Other commands

#### List all projects

```console
$ rmux list
```

#### Stop the session corresponding to a project

```console
$ rmux kill my_project
```

#### Delete a project

```console
$ rmux remove my_project
```

#### Debug session creation

```console
$ rmux debug my_project
```

Prints all the commands that are passed to `tmux source` to create the session, complete with hooks and everything.

You can save the output and use it directly without passing through `rmux`.
As long as the tmux server is already running. Also, it never attaches the session.

```console
$ rmux debug my_project | tmux source
```

#### Save current session as a project

```console
$ rmux freeze my_project
```

It will prompt you for confirmation before overriding an existing project, unless `--no-input` flag is passed.

You can also print the project file to stdout instead of opening a text editor:

```console
$ rmux freeze --stdout
```

## Copyright

Copyright (c) 2020 Saïd Dermoumi. See LICENSE for further details.
