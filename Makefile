.PHONY: all build deploy run monitor clean

all: build deploy run

build:
	@echo "Building Rust engine..."
	cd rust-engine && cargo build --release
	@echo "Compiling contracts..."
	npx hardhat compile

deploy:
	@echo "Deploying contracts..."
	npx hardhat run scripts/deploy.js --network mainnet

run:
	@echo "Starting infrastructure..."
	docker-compose up -d
	@echo "Starting ML optimizer..."
	python ml-optimizer/server.py &
	@echo "Starting MEV engine..."
	./rust-engine/target/release/mev-arbitrage-engine

monitor:
	@echo "Opening monitoring dashboard..."
	open http://localhost:3000

clean:
	cargo clean
	rm -rf artifacts cache
	docker-compose down

test:
	cargo test --release
	npx hardhat test