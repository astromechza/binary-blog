<meta x-title="Git tag-based auto-versioning scheme"/>

**Versioning is difficult.** I'd partly argue that this is the case because there are just so many different methods and strategies!

**NOTE**: if you want to jump straight to the code, click [here](#code).

There is no right answer that fits all software projects, [Semantic Versioning](https://semver.org/) is an attempt at this, but often doesn't fit well for large projects that may have a release cycle as long as 3-6 (even 12) months for a new version (eg: Android, iOS, Windows, etc).

However SemVer can work _very_ well for small libraries and distributables for which the `MAJOR.MINOR.PATCH` archetype describes changes in the project very well. It's an obvious choice for languages like Python, who's packaging mechanism requires a version number.

This post will mostly be concerned with small projects that may be distributed to an audience in which version numbers have some meaning.

### What information are we trying to convey

It's important to take a step back and remember what a version number is telling us (whether it is Semver or a date or a build number).

- What is the _latest_ version?
- What is the version I have, if I already have it installed?
- Is my version any different to the _latest_ version?
- How often does this project release changes?
- Do I gain anything by upgrading?
- What is the magnitude of change that I expect between these versions?

If your audience (or developers) will not be concerned with these questions then you probably don't need to worry about versioning at all!

### An evolution of my versioning practises

As I've gained more experience writing and maintaining software, my personal preference for versioning has changed and evolved (and is likely to keep doing so).

The first stop was just baking the version into the project files as `1.0.0` and never adjusting it _since I didn't have an audience concerned with the version_.

Then as I learnt about SemVer and saw it used in more places I began using it and attempting to remember to adjust the version file at fairly regular intervals. I only ever bumped the `MINOR` component (the second digit) and generally did so either too often or with no respect to the real SemVer spec.

In some projects I used just 2 digits of semver (`MAJOR.MINOR`) since I was never using the third digit!

I still kept having silly events where I'd intend to fix a bug or add a feature and yet I hadn't bumped the version number which often played havok with CI pipelines overwriting artifacts when not intended. I'd then have to go back and add a new commit with the bumped version info.

To get around that I began providing the version using Git tags. This is a commonly used strategy in many projects these days:

- Don't store the version in the project source code
- Add a Git tag like `v1.2.3` when you cut a new version
- Use your build/deployment framework to access the latest version tag and use it for the build
- Simply add a new git tag when its time to release a version
- If your Git provider supports it, protect the tag pattern `v\d+\.\d+\.\d+` so that it can only be used on master and can only be pushed by certain users.

This made managing the version pretty nice but still presented the problem in my projects that I'd never end up updating the `PATCH`/`BUILD` number (the 3rd digit).

This brings us to my current preferred way of doing version managment. **Forewarning: it uses a lot of Git**.

### The technique

Briefly:

- Use Git tags to store the 2 digit version tag (`v1.2`)
- The version of any commit, is the most recent, and highest version tag found for any parent commit on the master branch
- The 3rd number (the PATCH or BUILD) is _dynamically_ calculated as the distance in "merge" commits since the version tag (this can be relaxed to be any type of commits if you're not using merge commits)
- Add a `-devX` suffix if Git is not clean, or the distance to the last parent commit on the master branch is > 0

**Note:** A "merge" commit is one which has more than one parent. Usually as a result of a `git merge` or something.

### How it's implemented in a project

In some crazy world you could get away with implementing this in a `Makefile`. However, this is not that world so I prefer to write it as a broadly compatible Python script that a `Makefile` can call if necessary.

You can then use the `ver.py` file to access the version number when you need it during build and deployment. An example of the version progression may be:

```
$ for r in $(git rev-list HEAD); do git --no-pager show $r -s --oneline; python ver.py --ref $r; done
365a400 (HEAD -> feature-D) My most recent commit
0.1.0-dev1
3789ecb (origin/master, master, v0.1) Merge feature-C to master
0.1.0
f00386a (origin/feature-B) Work on feature C
0.0.13-dev1
37b46ef Merge feature-B to master
0.0.13
42a4fcf (origin/feature-B) More work on feature B
0.0.12-dev2
b93c2d1 Work on feature B
0.0.12-dev1
ba544ba Merge feature-A to master
0.0.12
...
```

It's a good idea to only deploy new artifacts when you're sure your building the release branch and that the version number has no `-devX` suffix indicating unmerged changes.

In a Python project, you could access and use the version number as follows in your `setup.py`:

```python
import imp 
import os
from setuptools import setup, find_packages

# you could run it via subprocess but this avoids the fork
version = imp.load_source('version', os.path.join(__file__, '../ver.py'))

setup(
    # ...
    version=version.get_version(),
    # ...
)
```

### Possible additions and extensions

- Always add `-devX` when you're not on the release branch
- When adding the `-devX` suffix, bump the `PATCH` digit in preperation for a new version (so that `1.2.3` < `1.2.4-dev` < `1.2.4`)
- Add a command to bump the minor or major versions (publish a new git tag based on the last one)
