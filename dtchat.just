run socket="tcp" instance="1" delay="":
    #!/bin/bash
    # Validation des paramètres
    if [[ "{{socket}}" != "tcp" && "{{socket}}" != "udp" ]]; then
        echo "❌ Erreur: socket doit être 'tcp' ou 'udp', reçu: {{socket}}"
        exit 1
    fi

    if [[ "{{instance}}" != "1" && "{{instance}}" != "2" ]]; then
        echo "❌ Erreur: instance doit être '1' ou '2', reçu: {{instance}}"
        exit 1
    fi

    echo "🚀 Démarrage de l'instance DTChat avec config: db/local/{{socket}}-{{instance}}.yaml"
    export DTCHAT_CONFIG=db/local/{{socket}}-{{instance}}.yaml
    if [ -n "{{delay}}" ]; then
        export DTCHAT_ACK_DELAY={{delay}}
        cargo run --features ack-delay
    else
        cargo run
    fi