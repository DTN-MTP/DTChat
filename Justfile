# Variables
rust_version := "stable"
target_dir := "target"

default:
    @just --list

fmt:
    cargo fmt --all
    @echo "✨ Code formaté"

check-fmt:
    @echo "🔍 Vérification du formatage..."
    cargo fmt -- --check

clippy:
    @echo "🔍 Analyse clippy en cours..."
    cargo clippy --all-targets --all-features -- -D warnings

clippy-fix:
    cargo clippy --fix --allow-dirty 
    @echo "🔧 Clippy a corrigé les problèmes détectés  "

pre-commit:
    #!/bin/bash
    set -e
    echo "🔍 Vérifications pre-commit..."
    
    if git diff --cached --name-only | grep -q "\.rs$"; then
        echo "📝 Fichiers Rust détectés, vérification de la qualité..."
        just check-fmt
        just clippy
    else
        echo "ℹ️  Pas de fichiers Rust modifiés, pas de vérifications nécessaires"
    fi
    
    echo "✅ Vérifications terminées!"
    
# Installe le hook git pre-commit
_setup-hook:
    #!/bin/bash
    echo "🔧 Installation du hook pre-commit..."
    cat > .git/hooks/pre-commit << 'EOF'
    #!/bin/bash
    set -e
    echo "🚀 Executing pre-commit checks..."
    just pre-commit
    EOF
        chmod +x .git/hooks/pre-commit
        echo "✅ Hook pre-commit installé! Il sera exécuté avant chaque commit."

# Désinstalle le hook git
remove-hook:
    rm -f .git/hooks/pre-commit
    @echo "🗑️ Hook pre-commit supprimé"

# Vérifie si le hook est installé
check-hook:
    #!/bin/bash
    if [ -f .git/hooks/pre-commit ]; then
        echo "✅ Hook pre-commit est installé"
        cat .git/hooks/pre-commit
    else
        echo "❌ Hook pre-commit n'est pas installé"
        echo "Lancez 'just setup' pour l'installer"
    fi

# Workflow complet pour nouveau développeur
hook-setup: _setup-hook
    @echo "🎯 Configuration terminée pour le développement"
    @echo "Commandes utiles:"
    @echo "  just fmt         - Formate le code"
    @echo "  just clippy      - Analyse le code"
    @echo "  just pre-commit  - Vérifie tout avant commit"

# Commande pour vérifier la qualité du code
quality: check-fmt clippy
    @echo "💎 Vérification qualité terminée"