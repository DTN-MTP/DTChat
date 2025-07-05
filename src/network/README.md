# Module Network

## Architecture

### 🏗️ Composants Principaux

#### 1. **NetworkEngine** (Façade)
- **Rôle** : Point d'entrée unifié pour toutes les opérations réseau
- **Responsabilités** :
  - Coordination entre composants
  - Interface simplifiée pour l'application
  - Gestion du cycle de vie

#### 2. **PeerManager** (Gestionnaire de Pairs)
- **Fichier** : `src/network/peer_manager.rs`
- **Responsabilités** :
  - Gestion de la liste des pairs
  - Ajout/suppression de pairs
  - Recherche de pairs par UUID
  - Gestion du pair local

#### 3. **MessageRouter** (Routeur de Messages)
- **Fichier** : `src/network/message_router.rs`
- **Responsabilités** :
  - Envoi de messages vers des pairs spécifiques
  - Envoi de messages vers des endpoints
  - Gestion des accusés de réception
  - Logique de routage

#### 4. **NetworkMonitor** (Moniteur Réseau)
- **Fichier** : `src/network/monitor.rs`
- **Responsabilités** :
  - Surveillance de la santé du réseau
  - Collecte de statistiques
  - Vérification de l'état des connexions

#### 5. **LoggingObserver** (Observateur de Logs)
- **Fichier** : `src/network/observers.rs`
- **Responsabilités** :
  - Logging des événements réseau
  - Implémentation du pattern Observer




## 🎯 Prochaines Étapes Recommandées

Pour externaliser complètement la couche réseau :

1. **Création d'une crate séparée** `network-core`
2. **Abstraction des types domaine** : Remplacer `ChatMessage` par des traits génériques
3. **Extraction du protobuf** : Déplacer la sérialisation protobuf vers l'application
4. **Interface générique** : Utiliser des traits pour les messages
