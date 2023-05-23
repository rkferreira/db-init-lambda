.SILENT:
local:
	cargo lambda build
	cargo lambda watch &
	cargo lambda invoke --data-file test/rds-creating-event.json

lambda:
	cargo lambda build --release --arm64
	cargo lambda deploy

check:
	rustc --version || echo "	Missing rust, install rustup. https://rustup.rs/ "
	cargo -V || echo "	Missing rust cargo. https://doc.rust-lang.org/cargo/getting-started/installation.html "
	cargo-lambda lambda --version || echo "	Missing cargo lambda. https://www.cargo-lambda.info/guide/installation.html "	

iam:
	aws iam create-role --role-name db-init-lambda-cargo-deploy --assume-role-policy-document file://aws-lambda-role.json
	aws iam wait  role-exists --role-name db-init-lambda-cargo-deploy
	aws iam attach-role-policy  --policy-arn  arn:aws:iam::aws:policy/AmazonRDSReadOnlyAccess --role-name db-init-lambda-cargo-deploy
	aws iam attach-role-policy  --policy-arn  arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole  --role-name db-init-lambda-cargo-deploy
	aws iam attach-role-policy  --policy-arn  arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole  --role-name db-init-lambda-cargo-deploy
	aws iam put-role-policy --role-name db-init-lambda-cargo-deploy --policy-name SecretsManagerAccess  --policy-document file://aws-lambda-policy.json

eventbridge:
	aws events put-rule --name "db-init-lambda" --event-pattern "{\"source\":[\"aws.rds\"],\"detail-type\":[\"AWS API Call via CloudTrail\"],\"detail\":{\"eventSource\":[\"rds.amazonaws.com\"],\"eventName\":[\"CreateDBInstance\"]}}"
