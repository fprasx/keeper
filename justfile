init:
    mkdir -p ~/.local/share/keeper
    git -C ~/.local/share/keeper init
    cp -R keeper-todo/wp.workflow ~/.local/share/keeper

install target:
    cargo build \
        --release \
        --bin {{target}} \
        && cp target/release/{{target}} ~/.local/bin/{{target}} \
        && chmod +x ~/.local/bin/{{target}}
