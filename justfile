install:
    cargo build --release \
        && cp target/release/keeper-todo ~/.local/bin/keeper-todo \
        && chmod +x ~/.local/bin/keeper-todo
