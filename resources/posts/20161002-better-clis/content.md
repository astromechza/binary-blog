<meta x-title="Building better command-line tools and applications"/>

### 1. Always use a proper option parser.

At first you might only need one or two 'positional' arguments, but soon as you
start wanting to support 'optional' arguments, usage, help, version info etc..
you soon find that using a pre-built option parser will make it far easier to
handle everything from automatic help generation, type/range checking, choices,
optionals, subcommands, and almost anything else.

Examples are:

- `optparse`, `argparse`, `click` for python
- `flag` for Golang

#### 1.1 Always respond to `--help`

When a user finds an odd executable or script they usually want to find out what
it does and what it can be used for. The user will be hesitant to run the
executable without any arguments as this might still do something! So even if
your cli doesn't support any arguments it's generally worth while responding
correctly to `--help` and providing information such as usage, description,
arguments allowed etc.

It's also a good idea to add a project url to the help text so that users can
get help, upgrade, or contribute improvements.

#### 1.2 Respond to `--version`

Unless your executable will only be written once and will not need to be
maintained or distributed you probably have some concept of "version". Whether
it is a semantic version number, git commit, build date, or build name it's
important to allow a user (or yourself) to extract this information from the
distributed executable. My personal preference is to use a '0.0.0' version number
and a Linux epoch datetime which is then overriden with the real details when
making an official release build.

I like to have something like the following:

```
$ thing --version
Version: <version language> (<git hash>) at <date time>
```

#### 1.3 Provide an option for machine-readable output

If it's a tool that generates some output, you can make it easier to pass the
results into further processes by switching to machine readable output.

CSV and JSON formats are good for this sort of thing. To allow people
to run `head`, `tail`, `grep` as a step in a pipeline, output a single csv, or
json structure per line. (`--json` or `--csv`)

Remember that if you're doing structure output, you can't intefere with it by
printing status messages or progress alerts to `stdout`, you'll need to push
everything to `stderr`.

For example, if the normal output is:

```
Animal              Legs
Lion                4
Ostrich             2
Spider              8
```

Then a structured output could be:

```
animal,legs
Lion,4
Ostrich,2
Spider,8
```

Or

```
{"animal": "Lion", "legs": 4}
{"animal": "Ostrich", "legs": 2}
{"animal": "Spider", "legs": 8}
```

#### 1.4 If you take a file as input, consider also accepting `-`

Quite a few tools accept `-` as an alias for `stdin` when they take a file as
input (for example `vi -` will open a file containing the information from
`stdin` stream). Consider supporting this as well so that users can pipe
content to your executable.

### 2. Don't exit with code 0 if there's an error!

This is critical. If you exit normally when an error occurs, a script that calls
it may incorrectly assume that the result was correct.

You can exit with pretty much any code from `0` to `255`. `0` always indicates success
while anything else is seen as a failure. Some codes should be avoided as they
are often used by Bash to indicate other issues. For example `127` usually means
that the called executable was not found and `126` means the user did not have
permission to execute the given item. See [exitcodes.html](http://tldp.org/LDP/abs/html/exitcodes.html).
Usually stick to `1` for general errors and `2` for issues regarding incorrect
arguments or configuration.

### 3. Use stderr when necessary

Your process has both `stdout` and `stderr` output streams. Although these are
separate streams that can be redirected by the user, they are mostly just interleaved
in the user's terminal. Use these wisely and give the user the power of picking
which parts of the output streams they care most about. Print errors and warnings
to stderr while keeping your traditional output on the stdout stream.

The stderr stream can be useful when doing machine-readable output. You may want
to print warnings and errors without corrupting the structured output on stdout.
You can use stderr for this and the user can split out the structured data from
the error stream.

### 4. Use appropriate tense and detail in your error messages

If an error occurs, **always** include a reason, and if you can suggest a
solution if the user is able to fix it.

```
USELESS: could not open file!
USEFUL:  could not open file "/bob/john": it does not exist!
```

Use _present_ tense for permanent errors and _past_ tense for more temporary
issues that might not occur next time:

```
TEMPORARY: could not open file "/bob/john"
PERMANENT: cannot open file "!*&^*!&#$"
```

For invalid values, mention *why* and *how* it was invalid:

```
BAD:    value was out of range
BETTER: value 10532 was out of range [0 -> 100]
BETTER: could not parse "837af" as integer
BETTER: expected Integer argument, got String
```

Consider adding suggestions:

```
An error occured, the file '/something/else' was not readable. Make sure you
have the correct permissions to read it.
```

It may be longer, but it certainly provides much more useful information for a
user.

### 5. You can do some great things with ANSI control codes

[ANSI](https://en.wikipedia.org/wiki/ANSI_escape_code) escape codes can be used
to control formatting, color, and other output options of Linux terminals. To
encode this information, certain sequences of bytes are embedded into the text
which specific terminals extract and interpret.

The most commonly used codes are those controlling the colour of the output but
other commonly supported options are background colour, intensity (bold/normal)
and underline.

Use coloured output carefully, it's a useful way of making output easier
for humans to parse but can often make it over-the-top gaudy.

One of the basic things can be printing anything to stderr in red.

Other crazy things:

- set window title
- blink text
- BEL alert tone
- arbitrary RGB colours

### 6. Extended ASCII and unicode

If your terminal supports it, you can use the [extended ascii](http://www.theasciicode.com.ar/)
and unicode characters to improve the formatting of your output.

For example you can print tables using the box lines:

```
╔══════╦═════╗
║ A    ║ B   ║
╠══════╬═════╣
```

And almost anything you'd like if you're allowed to use unicode. Remember that
these character encodings aren't supported by all terminals on all platforms and
that other tools like `grep` may not work correctly with multi-byte characters.

Also remember that it makes although it might make your output prettier, it
will intefere with people processing the output using `cut`/`awk`/`sed` etc..

### 7. Character rewriting and progress bars

Quite a few utilities like `curl`, `wget` etc.. use progress bars to convey the
status of an operation. It's a useful thing to add to your own tools if it may
perform a long-running operation. To this, use the `\r` carriage return or `\b`
backspace to delete and rewrite printed characters.

Example in Python:

```python
import sys
import time

width = 40
for i in xrange(width):
    sys.stdout.write("\r")
    sys.stdout.write("#" * i)
    sys.stdout.write(" " * (width - i))
    sys.stdout.write(" %.1f%%" % (i * 100.0 / width))
    sys.stdout.flush()
    time.sleep(0.1)

sys.stdout.write("\r")
sys.stdout.write("#" * width)
sys.stdout.write(" 100.0%\n")
```

**Be aware** that the backspaces don't have any effect when you're not in a
p/tty environment! This is why piping a progress bar ends up with hundreds of
lines of intermediate progress. See the next point on how to avoid this!

### 8. Detect P/TTY vs dumb environment

If you're using ANSI control codes, `\r` carriage return or `\b` backspace
or redrawing portions of lines, use TTY detection so that you get the nice
pretty output when a human is behind the controls but degrade to useable
output when piping it into a file, or further processing.

It's always a horrible experience reading through a log file containing sudden
ANSI codes.

### 9. Detecting the terminal size

Editors like `vi`, `nano` etc expand to fill the full size of the terminal, they
do this by using ANSI control codes to clear the screen and to write characters
at arbitrary positions.

You might also find useful moments to use the width of the terminal as a limit
for things like progress bars or tables.

Some terminals provide the `$LINES` and `$COLS` variables, other tools use the
`SIGWINCH` signal that indicates that the terminal was resized. Other languages
can use their own libraries to extract this information:

```python
import fcntl
import termios
import struct
import errno

def terminal_size(default_width=100, default_height=50):
    try:
        h, w, hp, wp = struct.unpack('HHHH', fcntl.ioctl(1, termios.TIOCGWINSZ, struct.pack('HHHH', 0, 0, 0, 0)))
        return w, h
    except IOError as e:
        if e.errno == errno.ENOTTY:
            return default_width, default_height
        raise
```

Again, remember that this only makes sense in a TTY interface.

### 10. If environment variables are affecting the execution, TELL THE USER

Something else that is a pet peeve of mine is scripts that use an environment
variable to silently change the way they operate. It's really useful for it to
declare when an environment variable is overriding the default settings or
workflow.

```
$ do-the-thing
$DOTHETHING_OPT = 12 was declared, doing the thing 12 times.
...
```
