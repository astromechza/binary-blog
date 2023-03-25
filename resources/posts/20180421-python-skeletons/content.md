<meta x-title="Python Skeletons"/>
<meta x-description="(Imported from old blog) Using spiro for generating simple code repositories based on a template."/>

I write a lot of Python. _Sometimes I even get paid for it._ Most of the time, it’s quick libraries and utilities focused on solving small and specific problems for myself and my general team. As I’ve been doing this, I find myself often wasting time or yack-shaving while I find the same old Hackernews solutions for various setuptools issues or testing flavours. To combat this, I reach for templated project skeletons that I can customise and generate in a single command. For Python, my requirements are generally:

- "Modern" setuptools **packaging** and integration
- Ability to generate **cli entrypoints** (for command line tools; this should be able to be skipped when generating a library)
- Solid examples of **pytest** integration (fixtures, parametrize(), etc..). I don’t want to have to look these up every time. Code coverage!
- The generated project should be able to be built, tested, and should have good Python style from the get go.
- Python 2/3 **compatible** to various extents. This is only because I’ve had to write a lot of Python 2.6/2.7 code 😲. Hopefully one day I can move exclusively to >3.5.
- A **README.md** with valid and reproducible instructions.
- Version-control based **versioning** via `setuptools_scm`. This makes it easy to cut versions using tags, rather than synchronising a tag and a version string in the repo.
- Travis-CI file for getting going with Travis (optional depending on whether the project is on Github). This should be able to be adapted for Gitlab-CI, or whatever CI is being used.

**TLDR**: _I want to know all the project setup stuff is taken care of and that I can build a solid project without duplicated effort._

## The Templater

I use a project of my own creation called Spiro for generating projects from a template directory. You could use any other template/skeleton system but Spiro fullfills a bunch of use cases for me:

- Good logic support in templates (if-else..) using Golang’s text/template package
- Templating of file/directory names
- Generate a new project in a single command
- Processors for various things like lower()/upper()/regex..

See the Spiro project for more examples: https://github.com/AstromechZA/spiro. I maintain it and add features generally when required to support new project templates.

## The Template/s

The template I use is available here: https://github.com/AstromechZA/python-package-spiro. At the time of writing it needs to be cloned or extracted onto your machine.

It has a `spec.yaml` that looks something like this:

```
package_name: hello-world
module_name: hello_world

author: Change Me
author_email: changeme@example.com

short_description: Hello world example project

use_scm_versioning: true

entrypoints:
    - hello-world
```

And with Spiro I can use it as follows:

```
$ ./spiro -edit python-package-spiro/\{\{.package_name\}\} python-package-spiro/spec.yaml some-output-dir
```

💡The `\{\{..` betrays the use of Golang templating, hopefully I can find a better way of autodetecting the template root in the future.

Which can easily be aliased to just:

```
$ alias new-python-project="./spiro -edit python-package-spiro/\{\{.package_name\}\} python-package-spiro/spec.yaml"
$ new-python-project /home/projects/
```

This command, with the `-edit` flag, opens up your `$EDITOR` to edit the spec inline before rendering the template.

## The Results

Running this template with the default “hello-world” spec contents gives the following:

- `python setup.py develop` sets up the package, installs setup and install dependencies.
- `python setup.py test` runs `pytest` unittests with examples of fixtures and parametrisation.
- `pip install .[test]` demonstrates the use of setuptools extras and installs the test dependencies in the virtualenv.
- A flake8 unittest runs style checking on the code base as part of the unittest. _A good way to ensure you’re sticking to the pep/community rules._
- A single cli command hello-world is made available as the entry point of the program.
- `setuptools_scm` means that versioning comes from tags in Git such as v0.1 or 1.2.3. This makes releasing new versions much easier. By default the version is seen as 0.0.0. (This does rely on doing a `git init` first).
- A Travis-CI file means that the project is ready to go with Travis running a suite of tests from the first time code is committed to the repo. Some small additions can give you automated Github or Pypi uploads as well.

## Conclusions

With this in place, you can jump straight into prototyping and releasing a Python-based tool without too much worry about its setup and packaging.

The approach can also be adopted for any other language since the patterns are language agnostic. I’ve written more complex templates for large Twisted-based python micro-services for work, but the most useful results always come from more frequently generated templates.

##Future work and TODOs

The template will always be tweaked over time as I work out more suitable ways of doing things or when new techniques, libraries, Python versions, etc come about. I also want to add some features to Spiro to make distribution of templates easier and to allow better sharing without having to manually git clone.
