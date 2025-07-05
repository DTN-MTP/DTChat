mod dtchat
target_dir := "target"

# =====================
# Quality tests
# =====================
# Commande par défaut
default :
    @echo "🚀 Bienvenue dans le projet DTChat!"
    @echo "Utilisez 'just setup' pour configurer l'environnement de développement"
    @echo "ou 'just help' pour voir les commandes disponibles"

# Formatage du code
fmt:
    @echo "Formate tout le code Rust du projet"
    cargo fmt --all
    @echo "✨ Code formaté"

# Vérification du formatage
check-fmt:
    @echo "🔍 Vérification du formatage..."
    cargo fmt --all -- --check

# Analyse statique (Clippy)
clippy:
    @echo "🔍 Analyse clippy en cours..."
    cargo clippy --all-targets --all-features -- -D warnings

# Correction automatique des problèmes Clippy
clippy-fix:
    @echo "Tente de corriger automatiquement les problèmes détectés par Clippy"
    cargo clippy --fix --allow-dirty 
    @echo "🔧 Clippy a corrigé les problèmes détectés"

# Vérification qualité avant commit
pre-commit:
    #!/bin/bash
    set -e
    echo "Vérifications pre-commit (formatage et clippy si fichiers Rust modifiés)"
    echo "🔍 Vérifications pre-commit..."
    
    if git diff --cached --name-only | grep -q "\.rs$"; then
        echo "📝 Fichiers Rust détectés, vérification de la qualité..."
        just quality
    else
        echo "ℹ️  Pas de fichiers Rust modifiés, pas de vérifications nécessaires"
    fi
    
    echo "✅ Vérifications terminées!"

# Vérification qualité globale
quality: check-fmt clippy
    @echo "Vérifie le formatage et la qualité du code (Clippy)"
    @echo "💎 Vérification qualité terminée"

# =====================
# Hook
# =====================
# Gestion du hook pre-commit
_setup-hook:
    #!/bin/bash
    echo "Installe le hook pre-commit dans .git/hooks"
    echo "🔧 Installation du hook pre-commit..."
    cat > .git/hooks/pre-commit << 'EOF'
    #!/bin/bash
    set -e
    echo "🚀 Executing pre-commit checks..."
    just pre-commit
    EOF
        chmod +x .git/hooks/pre-commit
        echo "✅ Hook pre-commit installé! Il sera exécuté avant chaque commit."

remove-hook:
    @echo "Supprime le hook pre-commit"
    rm -f .git/hooks/pre-commit
    @echo "🗑️ Hook pre-commit supprimé"

check-hook:
    #!/bin/bash
    echo "Vérifie si le hook pre-commit est installé et affiche son contenu"
    if [ -f .git/hooks/pre-commit ]; then
        echo "✅ Hook pre-commit est installé"
        cat .git/hooks/pre-commit
    else
        echo "❌ Hook pre-commit n'est pas installé"
        echo "Lancez 'just setup' pour l'installer"
    fi

# Setup du hook et rappel des commandes utiles
setup-hook: _setup-hook
    @echo "Configure le hook et affiche les commandes utiles"
    @echo "🎯 Configuration terminée pour le développement"
    @echo "Commandes utiles:"
    @echo "  just fmt         - Formate le code"
    @echo "  just clippy      - Analyse le code"
    @echo "  just pre-commit  - Vérifie tout avant commit"


# =====================
# Affiche l'aide personnalisée du projet
# =====================
help:
    @echo "\033[36m🚀 Welcome to DTChat Justfile!\033[0m"
    @echo "\n\033[33m📌 Assure la qualité du code Rust avec formatage et Clippy :\033[0m"
    @echo ""
    @echo "  just fmt         - Formate tout le code Rust du projet"
    @echo "  just check-fmt   - Vérifie le formatage du code"
    @echo "  just clippy      - Analyse statique avec Clippy (bloque sur warning)"
    @echo "  just clippy-fix  - Corrige automatiquement les problèmes Clippy"
    @echo "  just quality     - Vérifie formatage et qualité (Clippy)"
    @echo "\n\033[33m🔗 Gestion du hook pre-commit :\033[0m"
    @echo ""
    @echo "  just setup-hook  - Installe le hook pre-commit"
    @echo "  just remove-hook - Supprime le hook pre-commit"
    @echo "  just check-hook  - Vérifie si le hook pre-commit est installé"
    @echo "\n\033[33m📡 DTChat Commands:\033[0m"
    @echo ""
    @echo "  \033[32mjust dtchat run\033[0m \033[36m<socket>\033[0m \033[36m<instance>\033[0m \033[36m<delay>\033[0m"
    @echo "      → Démarre DTChat avec paramètres spécifiques"
    @echo "      → \033[36msocket\033[0m   : \033[33mtcp\033[0m|\033[33mudp\033[0m - Type de connexion"
    @echo "      → \033[36minstance\033[0m : \033[33m1\033[0m|\033[33m2\033[0m - Numéro d'instance"
    @echo "      → \033[36mdelay\033[0m    : Délai en ms pour ack-delay (vide = désactivé)"
    @echo "      → \033[90mEx: just dtchat run tcp 1 2000\033[0m (active ack-delay)"
    @echo "      → \033[90mEx: just dtchat run udp 2\033[0m (sans ack-delay)"
