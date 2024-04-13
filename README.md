# binary-blog

My blog! Written as a single big [main.rs](src/main.rs) file which I've used to learn Rust.

- Staging environment: [bensblog-staging.meierhost.com](https://bensblog-staging.meierhost.com/).
- Production environment: [bensblog.meierhost.com](https://bensblog.meierhost.com/).

```
make test
make build
make launch
```

Note that `launch` uses [Score](https://score.dev/) to run in Docker compose and the same [score.yaml](score.yaml) file is uploaded to Humanitec for the release environment.
