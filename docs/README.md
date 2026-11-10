# Documentation de conception

Cette documentation définit la direction produit et technique de *Submarine Game*. Elle distingue le prototype actuellement présent dans le dépôt des fonctionnalités visées.

## Documents

Pour prendre en main le jeu : **[Guide joueur — Armement et première attaque](guide-armement.md)** (lecture de l'écran, procédure de tir et dépannage).

Pour valider la riposte et les dégâts : **[Recette M5.2](recette-m5.2.md)**.

Pour valider la récupération après une attaque : **[Recette M5.3](recette-m5.3.md)** (pompes, isolation, incendies, pannes et bot Ingénierie).

## Dernier incrément — Courses de torpilles et explosions

**P4 : visualisation des courses estimées et perception des explosions** est implémenté : pointillés et interception supposée du tube sélectionné sur la carte, torpilles estimées avec traînée, temps/autonomie, son désactivable des explosions entendues et gerbes temporaires au périscope. Aucun impact caché ni navire détruit n'est confirmé gratuitement. Voir la **[recette P4](recette-p4.md)** ; le rendu navigateur/téléphone et l'audio réel restent à valider.

## Conduite de tir visuelle

Le bouton **POSTE DE COMMANDE**, sous le réglage sonore, ouvre les commandes Périscope, Pilote et Armement. Il devient **REVENIR A LA CARTE** lorsque le panneau est ouvert ; sa largeur est réservée pour maintenir un libellé visible.

L'écran Capitaine sépare les commandes en trois blocs titrés et bordés : **Périscope — Observer**, **Pilote — Manœuvrer** et **Armement — Préparer l'attaque**. Les deux panneaux d'ordres sont côte à côte à partir de 960 px et empilés en dessous. Les hypothèses locales et la désignation transmise sont affichées séparément. Cette réorganisation ne change ni les règles de tir ni le protocole v13 ; la recette visuelle/tactile reste à effectuer.

**P3 : attaque depuis des estimations visuelles manuelles** est implémenté avant M6. Le Capitaine sélectionne un contact au périscope, règle distance/cap/vitesse supposés et applique explicitement sa désignation. L'Armement humain ou bot prépare le tir sans piste sonar ; autorisation et procédure des tubes restent nécessaires. Voir la **[recette P3](recette-p3.md)**. La recette navigateur/téléphone et l'équilibrage restent à confirmer ; télémètre et estimation assistée sont différés.

## Incréments périscope précédents

M5 est clôturé par décision du porteur du projet. **P1 : observation et désignation visuelles au périscope** est implémenté avant la mission complète M6 : vue à silhouettes 2D, usage en mouvement à profondeur périscopique et contacts visuels autonomes partageables. Le joueur solo est obligatoirement Capitaine, les quatre autres postes étant des bots. Le partage visuel ne permet pas de préparer directement un tir. Voir la **[recette P1](recette-p1.md)** ; l'essai sur téléphone réel et l'équilibrage restent à confirmer.

**P2 : détection du périscope** est implémenté : vigies sur tous les bâtiments à flot, cargos moins efficaces, acquisition progressive et transmission privée à l'escorte. Aucun avertissement ni indicateur de détection n'est ajouté à l'interface. Voir la **[recette P2](recette-p2.md)** ; la recette manuelle et l'équilibrage restent à confirmer. La détection de la coque en surface périscope rentré reste un suivi séparé.

Voir les [règles du périscope](game-design.md#65-détection-et-pistes) et le [découpage P1/P2 dans la roadmap](roadmap.md#p1--périscope--observation-et-désignation-visuelles).

| Document | Objet | Statut |
|---|---|---|
| [Game Design Document](game-design.md) | vision, boucle de jeu, rôles, règles et expérience mobile | brouillon 0.1 |
| [Architecture](architecture.md) | architecture actuelle, cible, scénarios, qualités et risques | brouillon 0.1 |
| [Roadmap](roadmap.md) | vertical slices, livrables et critères de validation | proposée 0.1 |

## Ordre de lecture

1. Lire le GDD pour comprendre le jeu à construire.
2. Lire l'architecture pour comprendre les contraintes et les frontières du système.
3. Utiliser la roadmap pour choisir le prochain incrément de développement.

## Règle de maintenance

- Une fonctionnalité cible n'est pas considérée comme livrée parce qu'elle est documentée.
- Tout changement majeur de règle met à jour le GDD.
- Tout changement de frontière, protocole ou autorité met à jour l'architecture.
- Tout changement de priorité ou de critère de sortie met à jour la roadmap.
- Une décision technique structurante et controversée doit être consignée dans un ADR dédié avant son implémentation.

## Direction retenue

- Univers fictif inspiré de la Seconde Guerre mondiale.
- Simulation poussée avec procédures adaptées au tactile.
- Jeu coopératif de un à cinq joueurs.
- Bots pour les postes vacants.
- Présentation entièrement 2D sur navigateur ordinateur et téléphone.
- Serveur autoritaire et simulation déterministe.

## Questions encore ouvertes

- Nom définitif du jeu et du sous-marin fictif.
- Direction artistique finale.
- Valeurs d'équilibrage après les premiers tests de mission.
- Navigateurs et appareils mobiles constituant la matrice de support officielle.
- Hébergement de la première démonstration publique.
