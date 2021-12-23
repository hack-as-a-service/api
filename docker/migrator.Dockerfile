# This Docker image performs migrations on the database

FROM nixpkgs/nix:nixos-21.11

WORKDIR /usr/src/app

RUN nix-channel --add https://nixos.org/channels/nixos-21.11 nixpkgs && \
    nix-channel --update
RUN nix-env -iA nixpkgs.diesel-cli

COPY ./migrations ./migrations

CMD ["/nix/var/nix/profiles/default/bin/diesel", "migration", "run"]
