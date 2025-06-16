# vim: set noexpandtab:
# setlocal noexpandtab softtabstop=0

APP_NAME := $(shell yq e '.package.name' Cargo.toml)
VERSION := $(shell yq e '.package.version' Cargo.toml)
IMAGE_TAG := $(shell [[ "${CIRCLE_BRANCH}" =~ ^master$$ ]] && echo "v${VERSION}" || echo "v${VERSION}-snapshot")

PLATFORM := linux/amd64
ECR_REPO := 783764586577.dkr.ecr.eu-west-1.amazonaws.com
DOCKER := podman
CACHE_IMAGE_NAME := cache

fmt:
	cargo fmt --all -- --check

lint: fmt
	cargo clippy --all-targets --all-features -- -D warnings

upgrade:
	cargo outdated
	cargo upgrade

up: lint
	cargo lambda watch

invoke:
	cargo lambda invoke lambda_sns_to_slack --data-example cloudwatch-alarm-sns-payload-single-metric

build:
	cargo lambda build --release

arm64_binary:
	rm -f target/lambda/lambda_sns_to_slack/bootstrap target/lambda/lambda_sns_to_slack/bootstrap.zip
	cargo lambda build --target aarch64-unknown-linux-musl --release
	cargo lambda build --target aarch64-unknown-linux-musl --release --output-format zip
	ls -sl target/lambda/lambda_sns_to_slack/

upx: ## compress the binary with upx
upx: arm64_binary
	upx --best target/lambda/lambda_sns_to_slack/bootstrap
	ls -sl target/lambda/lambda_sns_to_slack/

zip_build: upx
	cd target/lambda/lambda_sns_to_slack/ && zip --update ./bootstrap.zip ./bootstrap && ls -sl
	zip -sf target/lambda/lambda_sns_to_slack/bootstrap.zip

image_build: ## build image with registry cache
image_build: lint
	${DOCKER} build --file Dockerfile \
		--platform ${PLATFORM} \
		-t ${ECR_REPO}/${APP_NAME}:${IMAGE_TAG} .

help: ## Show this help
	@echo "\nSpecify a command. The choices are:\n"
	@grep -hE '^[0-9a-zA-Z_-]+:.*?## .*$$' ${MAKEFILE_LIST} | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[0;36m%-20s\033[m %s\n", $$1, $$2}'
	@echo ""

.PHONY: help

.DEFAULT_GOAL := help
