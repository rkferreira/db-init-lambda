# db-init-lambda
Lambda for RDS bootstrap with customizations


## Project creation

```
cargo lambda new db-init-lambda \
    && cd db-init-lambda
```


## Dev && Test

```
cargo lambda build                                              # local target
cargo lambda build --release --arm64                            # lambda arm target

cargo lambda watch                                              # local test
cargo lambda invoke --data-file test/rds-creating-event.json    # local test
```


## Deploy

```
cargo lambda deploy
```


