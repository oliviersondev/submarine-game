# Roadmap de conception et de développement

**Version :** 0.1  
**Date :** 1 septembre 2026  
**Statut :** active

Cette roadmap transforme le prototype réseau actuel en une mission de sous-marin jouable. Elle privilégie les vertical slices : chaque jalon doit relier règles, protocole, serveur et interface plutôt que terminer un système isolé invisible pour le joueur.

## 1. Principes de planification

- Construire d'abord une mission courte jouable par une personne avec des bots.
- Ajouter une seule nouvelle boucle de décision majeure par jalon.
- Tester les règles dans `simulation` avant de les présenter dans le client.
- Prototyper chaque interface sur une largeur de 360 px avant d'ajouter du détail.
- Conserver la simulation déterministe et l'autorité serveur à chaque étape.
- Ne pas engager le déploiement cloud avant que la mission de référence soit jouable localement.
- Revoir le GDD à la fin de chaque jalon à partir des tests de jeu.

## 2. État actuel

Le dépôt fournit déjà :

- quatre crates avec des responsabilités bien séparées ;
- un client Bevy compilable en WASM/WebGL2 ;
- un WebSocket binaire `postcard` ;
- un lobby global avec cinq rôles uniques ;
- une simulation autoritaire à 20 Hz ;
- validation des commandes par rôle ;
- navigation inertielle, plongée, endurance, bruit et interpolation du rendu ;
- convoi déterministe, écoute passive et active, pistes incertaines et classification progressive ;
- interfaces tactiles Pilote, Ingénierie, Sonar et carte tactique du Capitaine.

L'armement, les réparations et la réaction tactique de l'escorte sont encore symboliques. Les salles démarrent dès un humain et remplissent les rôles vacants avec des bots, mais ne gèrent ni persistance ni reprise de session.

## 3. Vue d'ensemble

| Jalon | Résultat jouable | Priorité |
|---|---|---:|
| M0 | documentation et baseline technique fiables | immédiate |
| M1 | une salle démarre avec un joueur et quatre bots | critique |
| M2 | le sous-marin manœuvre avec ressources et bruit | critique |
| M3 | le sonar découvre et suit un convoi | critique |
| M4 | une torpille peut atteindre une cible simulée | critique |
| M5 | une escorte cherche, attaque et provoque des avaries | critique |
| M6 | une mission complète possède victoire, défaite et bilan | critique |
| M7 | les cinq postes sont utilisables sur téléphone | élevée |
| M8 | reconnexion, robustesse et déploiement de démonstration | élevée |
| M9 | équilibrage, tutoriels et préparation de la première version | normale |

## 4. M0 - Baseline et documentation

**Objectif :** disposer d'une référence commune avant de modifier les règles et le protocole.

### Livrables

- GDD, architecture et roadmap relus.
- README principal relié à la documentation de conception.
- État actuel explicitement distingué de la cible.
- Commande de vérification de référence : `make check-all` puis `make test`.
- Liste des valeurs d'équilibrage provisoires identifiée dans le GDD.

### Critères de validation

- Les cinq rôles ont une responsabilité et une boucle de décision documentées.
- La mission de référence et ses conditions de fin sont définies.
- Les changements de code peuvent être rattachés à un jalon.
- La documentation ne présente pas les fonctions cibles comme déjà réalisées.

## 5. M1 - Salles, bots et protocole durable

**Objectif joueur :** lancer une partie seul ou en groupe sans attendre cinq connexions.

### Règles et domaine

- Un rôle libre est occupé par un bot.
- Un joueur peut prendre un rôle libre avant ou pendant le lobby.
- Le capitaine donne des ordres simples aux bots.
- Les bots maintiennent au minimum cap, vitesse et profondeur.

### Serveur et protocole

- Introduire une version de protocole.
- Ajouter identifiants de salle, session et commande.
- Remplacer le lobby global par un registre de salles.
- Ajouter état prêt et démarrage explicite.
- Créer la simulation avec une graine et une configuration de mission.
- Ajouter numéros de tick et de snapshot.
- Tester l'aller-retour `postcard` avec des fixtures versionnées.

### Client

- Créer ou rejoindre une salle par un code court.
- Afficher joueurs, rôles, bots et états prêts.
- Permettre au capitaine d'envoyer un ordre structuré à un bot.
- Construire l'URL WebSocket depuis l'origine ou une configuration de développement.

### Critères de validation

- Une personne lance une salle avec quatre bots.
- Cinq personnes peuvent toujours occuper les cinq rôles.
- Deux salles évoluent indépendamment dans le même processus.
- Un bot n'envoie que des commandes permises pour son rôle.
- Deux exécutions avec la même graine et les mêmes ordres atteignent le même état final sur la même cible.

### État implémenté au 27 août 2026

Les livrables M1 ci-dessus sont présents sous forme minimale : salles éphémères en mémoire, rôles uniques, prêts, démarrage explicite dès un humain avec quatre bots en solo, protocole v1 avec fixtures, mission déterministe numérotée et ordre du capitaine au bot Pilote. Les tests couvrent notamment deux salles isolées, les permissions du bot et le rejeu déterministe. La persistance, la reconnexion, les bots des autres postes et des contrôles capitaine configurables restent hors de M1 ; l'ordre tactile livré utilise une consigne fixe de démonstration.

## 6. M2 - Navigation, plongée et endurance

**Objectif joueur :** choisir entre vitesse, profondeur, autonomie et discrétion.

### Règles et domaine

- Séparer consignes et valeurs réelles de cap, vitesse et profondeur.
- Ajouter accélération, taux de virage et vitesse verticale.
- Ajouter états surface, profondeur périscopique et plongée.
- Ajouter ballasts simplifiés et remontée d'urgence.
- Ajouter diesels, moteurs électriques, batterie et oxygène.
- Calculer une signature acoustique qualitative.
- Centraliser les paramètres du bâtiment.

### Serveur et protocole

- Valider les nouvelles commandes du pilotage et de l'ingénierie.
- Projeter uniquement les mesures nécessaires à chaque poste.
- Ajouter événements de seuil : batterie faible, air critique, cavitation et profondeur critique.

### Client

- Remplacer les changements instantanés du pilote par consignes et instruments réels.
- Ajouter une première vue ingénierie : propulsion, batterie, oxygène et charge électrique.
- Afficher bruit propre et alertes partagées.

### Tests essentiels

- Conservation des bornes et absence de flottants non finis.
- Consommation électrique croissante avec le régime.
- Impossibilité d'utiliser les diesels sans prise d'air.
- Recharge en surface et décharge en plongée.
- Passage reproductible entre les états de plongée.
- Commandes tactiles réalisables sur 360 px de large.

### Critère de validation

Un joueur accompagné de bots peut quitter la surface, atteindre une profondeur donnée, naviguer silencieusement puis remonter avant épuisement de ses ressources.

### État implémenté au 28 août 2026

M2 est livré avec un protocole v2 sans compatibilité v1 : configuration centralisée du bâtiment, consignes distinctes des mesures réelles, inertie horizontale et verticale, états de plongée, ballasts, remontée d'urgence, propulsion diesel-électrique, batterie, oxygène, charge et signature acoustique qualitative. Le serveur valide les commandes Pilote et Ingénierie, automatise l'Ingénierie lorsqu'elle est tenue par un bot et produit des projections propres aux rôles. Le client fournit des postes tactiles Pilote et Ingénierie ainsi que le bruit et les alertes partagés. Les valeurs d'endurance restent provisoires jusqu'aux tests d'équilibrage ; la distribution électrique détaillée et les avaries restent prévues pour M5.

## 7. M3 - Détection et poste sonar

**Objectif joueur :** découvrir un convoi sans recevoir sa position exacte gratuitement.

### Règles et domaine

- Ajouter navires marchands et escorte avec trajectoires simples.
- Calculer signatures, propagation simplifiée et bruit ambiant.
- Produire des observations passives avec relèvement et incertitude.
- Créer, mettre à jour, dériver et expirer des pistes.
- Ajouter confiance et classification progressive.
- Ajouter sonar actif avec mesure de distance plus précise et révélation du sous-marin.
- Garantir que l'IA et les projections n'utilisent que les informations observables.

### Serveur et protocole

- Projeter observations et pistes selon le rôle.
- Partager explicitement une piste vers le capitaine et l'armement.
- Ne jamais transmettre l'état réel d'un ennemi non observé.

### Client

- Créer l'écran sonar tactile.
- Afficher relèvements, historique, confiance et classification.
- Permettre création, sélection, fusion et partage de piste.
- Créer la carte du capitaine avec zones d'incertitude.

### Tests essentiels

- Aucun contact hors portée ne produit d'observation.
- Le bruit propre réduit la qualité d'écoute.
- Une piste non observée perd de la confiance et dérive.
- Un ping améliore la mesure mais augmente la détection du joueur.
- Les projections réseau ne contiennent aucune entité cachée.

### Critère de validation

L'équipage localise, suit et classe un convoi en combinant écoute passive, manœuvre et éventuellement ping actif.

### État implémenté au 1 septembre 2026

M3 est livré sous forme minimale avec un protocole v3 sans compatibilité v2. La simulation crée deux cargos et une escorte sur une route déterministe, produit des observations passives bruitées sans distance exacte, puis construit des pistes dont la distance, le cap, la vitesse, la confiance et la classification se précisent progressivement. Les pistes dérivent et expirent en l'absence d'observation. Le Sonar peut pinguer, sélectionner, fusionner, abandonner et partager ses pistes ; le Capitaine et l'Armement ne reçoivent que les pistes partagées. Un bot Sonar partage automatiquement les pistes fiables. Le ping actif améliore les mesures et crée une piste privée imparfaite du sous-marin pour l'escorte, mais celle-ci ne change pas encore de comportement avant M5.

## 8. M4 - Solution de tir et torpilles

**Objectif joueur :** préparer une attaque dont la réussite dépend de la qualité des observations.

### Règles et domaine

- Ajouter tubes, portes, inondation, chargement et stock de torpilles.
- Ajouter cible désignée et paramètres de solution.
- Calculer un point d'interception à partir d'une piste, jamais de la cible réelle.
- Créer une entité torpille au lancement.
- Simuler trajectoire, autonomie, profondeur et collision.
- Appliquer des dégâts à la cible.
- Faire réagir le convoi à un tir ou à un impact observé.

### Serveur et protocole

- Remplacer `FireTorpedo { bearing }` par une procédure et des commandes explicites.
- Associer les accusés à l'identifiant de commande.
- Diffuser uniquement les torpilles observables par le rôle concerné.

### Client

- Créer l'écran armement tactile.
- Afficher les étapes de préparation d'un tube.
- Présenter stabilité et sources d'incertitude de la solution, sans pourcentage exact de succès.
- Demander une confirmation claire avant le tir.

### Tests essentiels

- Impossible de tirer un tube fermé, vide ou non préparé.
- Une piste imprécise produit une solution imprécise.
- Une torpille manque une cible qui change suffisamment de route.
- Un impact dépend de la trajectoire simulée.
- Le stock et les temps de rechargement restent cohérents.

### Critère de validation

Une torpille préparée à partir d'une piste stable peut toucher un cargo ; un tir précipité peut le manquer pour une raison explicable.

## 9. M5 - Escorte, attaque et contrôle des avaries

**Objectif joueur :** survivre à une riposte en arbitrant silence, manœuvre et réparations.

### Règles et domaine

- Implémenter les états escorte, suspicion, recherche et attaque.
- Donner à l'escorte ses propres observations et pistes.
- Ajouter une attaque par charges de profondeur ou arme équivalente fictive.
- Ajouter compartiments, intégrité, voies d'eau, incendies et pannes.
- Faire influencer l'inondation sur assiette et profondeur.
- Ajouter pompage, isolation et réparations temporisées.
- Ajouter propagation limitée des dégâts.

### Serveur et protocole

- Projeter les avaries détaillées à l'ingénierie et leur synthèse aux autres postes.
- Conserver les événements critiques jusqu'à confirmation par snapshot.
- Journaliser les causes de dégâts et de destruction pour le bilan.

### Client

- Compléter l'écran ingénierie avec compartiments et priorités.
- Ajouter alertes globales non exclusivement sonores.
- Afficher les ordres d'urgence du capitaine.
- Ajouter des retours clairs lors d'une commande bloquée par une panne.

### Tests essentiels

- L'escorte ne poursuit pas une position réelle qu'elle n'a pas observée.
- Une voie d'eau non traitée augmente l'inondation.
- Pomper sans réparer la fuite ne résout pas durablement l'avarie.
- Une panne bloque ou dégrade les commandes concernées.
- La profondeur critique aggrave les risques de coque.

### Critère de validation

Après avoir attaqué, l'équipage peut rompre le contact et survivre à une recherche ennemie en contrôlant bruit, profondeur et dégâts.

## 10. M6 - Mission de référence complète

**Objectif joueur :** jouer de bout en bout la mission définie dans le GDD.

### Contenu

- Briefing et configuration de difficulté.
- Zone d'opération, route du convoi et extraction.
- Cargo prioritaire, navires secondaires et deux escortes.
- Objectifs principal et secondaires.
- Limite de temps.
- Conditions de victoire et de défaite.
- Bilan expliquant détection, tirs, dégâts et ressources.
- Redémarrage avec une nouvelle graine.

### Critères de validation

- La mission est gagnable seul avec des bots en difficulté initiation.
- La mission est gagnable à cinq sans accès à une information omnisciente.
- Chaque poste prend des décisions utiles pendant l'approche, l'attaque ou l'évasion.
- Le bilan permet d'identifier pourquoi un tir a manqué ou pourquoi le sous-marin a été détecté.
- Une partie complète ne produit pas de valeur non finie ni de divergence de simulation détectée.

À la fin de M6, le projet possède son **MVP de gameplay**.

## 11. M7 - Interfaces mobiles complètes

**Objectif joueur :** utiliser chaque poste depuis un téléphone sans clavier.

### Travaux

- Unifier les cinq postes dans un shell responsive.
- Supporter portrait et paysage, avec paysage recommandé pour carte et sonar.
- Garantir des cibles tactiles d'au moins 44 pixels CSS.
- Supprimer toute dépendance au survol.
- Ajouter retour d'état pour chaque commande.
- Ajouter contrastes, symboles redondants et réduction des animations.
- Tester les changements d'orientation et le redimensionnement du canvas.
- Mesurer consommation mémoire, temps de chargement et cadence sur téléphone réel.

### Matrice de validation minimale

| Configuration | Attendu |
|---|---|
| 360 x 640 portrait | fonctions critiques accessibles sans recouvrement |
| 640 x 360 paysage | carte et sonar jouables |
| souris et clavier | raccourcis non obligatoires mais fonctionnels |
| tactile uniquement | mission complète réalisable |
| réseau mobile instable | état de connexion visible et reprise possible |

## 12. M8 - Reconnexion, robustesse et déploiement

**Objectif joueur :** rejoindre une démonstration sécurisée et reprendre une partie après une coupure.

### Réseau

- Ajouter jeton de reprise et période de grâce.
- Donner temporairement le rôle au bot lors d'une coupure.
- Resynchroniser par snapshot complet et séquence d'événements nécessaire.
- Gérer client lent, trame invalide et version incompatible.
- Borner taille et débit des commandes.

### Livraison web

- Produire une URL `wss://` correcte en production.
- Servir HTML, WASM et assets par HTTPS.
- Ajouter manifeste PWA et icônes si le mode installable est conservé dans la promesse produit.
- Ajouter un conteneur serveur minimal et un health check.
- Ajouter logs structurés et métriques de tick.
- Documenter sauvegarde inexistante : les parties restent éphémères.

### Critères de validation

- Une coupure de 20 secondes ne termine pas la mission.
- Le bot reprend puis rend le poste sans double commande.
- Un client incompatible reçoit une erreur compréhensible.
- Une page HTTPS n'effectue aucune requête en contenu mixte.
- Le serveur signale une salle dont les ticks dépassent leur budget.

## 13. M9 - Équilibrage et première version

### Travaux

- Organiser des tests à un, deux, trois et cinq joueurs.
- Ajuster durée de mission, ressources, bruit, précision et agressivité ennemie.
- Ajouter tutoriel de chaque poste et tutoriel d'équipage.
- Finaliser direction visuelle, sons et options d'accessibilité.
- Réduire les temps morts et identifier les responsabilités qui se chevauchent.
- Stabiliser les paramètres et les fixtures de protocole.
- Mettre à jour GDD, architecture, README et notes de version.

### Critères de sortie

- Aucun bloqueur connu sur navigateur ordinateur ou téléphone cible.
- Mission terminable avec toutes les compositions d'équipage prises en charge.
- Tutoriels suffisants pour un joueur sans connaissance navale.
- Résultat des actions et des échecs expliqué par l'interface.
- Vérifications et tests automatisés verts.

## 14. Définition de terminé

Une fonctionnalité n'est terminée que si :

- sa règle et ses unités sont documentées ;
- elle appartient à la bonne crate ;
- le serveur valide toutes ses commandes ;
- elle ne révèle pas d'information cachée au client ;
- ses transitions principales et ses bornes sont testées ;
- elle est utilisable sans clavier lorsqu'elle appartient au flux normal ;
- elle possède des états de chargement, blocage et erreur visibles ;
- `make check-all` et `make test` réussissent ;
- la documentation reflète le comportement réellement livré.

## 15. Travaux différés après la première version

- Plusieurs sous-marins alliés dans une même mission.
- Joueur contre joueur.
- Campagne persistante et progression d'équipage.
- Types de bâtiments et de torpilles supplémentaires.
- Météo, couches thermiques et propagation acoustique avancée.
- Matchmaking public et comptes permanents.
- Chat vocal intégré.
- Spectateurs et rejeux partageables.
- Déploiement horizontal de grande échelle.

Ces éléments ne doivent pas influencer la conception du MVP tant qu'un besoin concret ne l'exige pas.
