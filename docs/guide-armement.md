# Guide joueur — préparer et tirer une torpille (M4)

Ce guide décrit le poste **Armement** et les commandes associées du **Capitaine** dans le prototype actuel. Les durées et valeurs ci-dessous correspondent à la configuration par défaut.

## 1. Le principe

Une attaque se prépare à plusieurs postes :

1. Le **Sonar** écoute, construit des pistes et les partage.
2. Le **Pilote** place le sous-marin à une profondeur et une orientation compatibles avec le lancement.
3. L'**Armement** choisit une piste, prépare un tube et règle sa torpille.
4. Le **Capitaine** autorise la configuration du tube.
5. L'**Armement** confirme le lancement.

En solo au poste Armement, les autres postes sont tenus par des bots. Le bot Sonar partage ses pistes suffisamment fiables ; le bot Capitaine autorise les solutions disponibles. Le bot Pilote ne positionne pas automatiquement le bâtiment pour une attaque.

**Choisir une piste ne configure pas le tube. Charger un tube ne le rend pas prêt au tir.**

## 2. Lire la carte et les mesures

La carte représente les **estimations partagées**, pas les positions réelles des ennemis. Le sous-marin est au centre et le nord reste en haut : la carte ne tourne pas avec son cap.

| Affichage | Signification |
|---|---|
| `N 000`, `E 090`, `S 180`, `W 270` | Directions absolues : nord, est, sud, ouest. |
| `T6` sur la carte, `P6` dans les mesures | La même piste numéro 6. Ce n'est ni le tube 6 ni une torpille. |
| `Merchant` | Contact estimé marchand, à privilégier pour essayer une attaque de cargo. |
| `Escort` | Contact estimé escorte. Ce n'est pas une interdiction de tir. |
| `CAP 000 deg / 0.0 kn / PROF. 0 m` | Votre sous-marin : orienté au nord, immobile, en surface. |
| `P6 7 deg / 3878m / conf. 69%` | Piste sélectionnée : relèvement de 7° depuis votre position, distance estimée de 3 878 m et confiance de piste de 69 %. |
| `Cible cap 104 / 40.0 kn` | Route et vitesse **estimées de la cible**, et non celles de votre sous-marin. |

La **confiance n'est pas une probabilité de toucher**. Les estimations de distance, de cap, de vitesse et de classification peuvent être fausses. Plusieurs pistes peuvent aussi correspondre à un même contact : le Sonar peut les fusionner.

La carte est schématique : se fier aux valeurs textuelles pour les distances, plutôt qu'à une mesure en pixels sur l'écran.

## 3. Lire l'état d'un tube

| Affichage | Signification |
|---|---|
| `TUBE 4 AV` | Tube numéro 4, dirigé vers l'avant. Les tubes 1 à 4 sont avant ; le tube 5 est arrière. |
| `Charge / 0s` | Une torpille est chargée et le chargement est terminé. Le tube n'est pas forcément configuré, inondé ou ouvert. |
| `Reserve 1` | Il reste une torpille non affectée à un tube. Les torpilles en cours de chargement sont déjà retirées de la réserve. |
| `courses suivies 0` | Aucune course de torpille propre n'est actuellement suivie. |
| `autorisation NON` | Le Capitaine n'a pas autorisé cette configuration, ou l'autorisation a été annulée. |
| `SELECTION : P48` | Piste choisie localement ; elle n'est pas automatiquement appliquée au tube. |
| `CONFIGURATION APPLIQUEE : aucune cible` | Aucune configuration de tir n'a été acceptée pour ce tube. |
| `CONFIGURATION APPLIQUEE : P7 / torpille 4m / correction cap +0 deg / vit. +0 kn` | Le tube est configuré pour la piste 7, à 4 m, sans correction manuelle des estimations. |

Le stock initial comprend **cinq torpilles au total**, et non cinq torpilles par tube. Par exemple, `Reserve 1 / courses suivies 0` peut simplement signifier que quatre torpilles sont déjà affectées à des tubes, sans aucun lancement.

Les messages `SELECTION NON APPLIQUEE` et `REGLAGES NON APPLIQUES` signalent un écart entre votre choix local et le tube. Dans ce cas, le tir est suspendu : utiliser **Configurer** pour appliquer le changement, ou revenir aux réglages déjà appliqués. Sélectionner à nouveau le tube recharge ses réglages appliqués lorsqu'il est configuré. Une nouvelle configuration impose une nouvelle autorisation.

## 4. Procédure complète au poste Armement

### Étape 1 — Choisir la piste et le tube

- **Piste … >** passe à la piste partagée suivante.
- **Tube … >** passe au tube suivant.
- Pour débuter, préférer une piste `Merchant`, suffisamment suivie pour disposer d'une distance, d'un cap et d'une vitesse estimés.
- Utiliser un tube avant si la solution de tir est devant le sous-marin.

Ces boutons font défiler la sélection ; il n'est pas nécessaire de toucher les marqueurs de la carte.

### Étape 2 — Charger

Sur un tube vide, appuyer sur **Charger**, puis attendre **20 secondes**. Le statut passe de `Chargement` à `Charge`.

Si le tube est déjà chargé, passer directement aux réglages.

### Étape 3 — Régler et configurer

Le bouton qui affiche **Prof. torpille … m** fait défiler trois réglages. Les boutons moins et plus modifient le réglage affiché :

| Réglage | Effet des boutons | Effet sur le tir |
|---|---|---|
| `Prof.` | ±1 m, entre 1 et 15 m | Profondeur de la **torpille**, pas du sous-marin. Pour débuter contre un navire de surface, conserver 4 m. |
| `Cap cible` | Correction de ±5°, limitée à ±45° | Corrige la route estimée de la cible utilisée pour le calcul. Ne règle pas directement le cap de la torpille. |
| `Vit. cible` | Correction de ±1 nœud, limitée à ±10 nœuds | Corrige la vitesse estimée de la cible utilisée pour le calcul. Ne change pas la vitesse de la torpille. |

Exemple : une piste estimée à 40 nœuds avec une correction de vitesse de −10 nœuds sera calculée comme une cible à 30 nœuds. Cela peut permettre un calcul, mais ne prouve pas que cette correction est juste.

Appuyer sur **Configurer** pour envoyer la piste sélectionnée et les réglages au tube. Les modifications locales ne sont pas appliquées avant cet envoi.

Vérifier ensuite que `CONFIGURATION APPLIQUEE` affiche la piste voulue et qu'une solution apparaît. Si la configuration est refusée, le tube conserve sa configuration précédente, ou reste sans configuration.

### Étape 4 — Préparer le tube

Le même bouton change d'action selon son état :

1. **Inonder** : attendre **8 secondes**.
2. **Ouvrir** : attendre **3 secondes**.
3. Le tube devient `Ouvert / pret`.

Il faut une configuration acceptée avant l'inondation. Les opérations temporisées produisent du bruit et peuvent dégrader l'écoute du Sonar.

### Étape 5 — Vérifier l'autorisation et le positionnement

- Attendre `autorisation OUI` : le bot Capitaine s'en charge si le poste est vacant ; un humain utilise **Autoriser** sur le même tube.
- Le sous-marin doit être à **20 m de profondeur ou moins**.
- Le **cap d'interception calculé** doit être à ±90° de l'avant pour les tubes 1 à 4, ou de l'arrière pour le tube 5. C'est la direction du tir calculé qui compte, pas seulement le relèvement actuel du contact.

Une reconfiguration, une fermeture ou une purge annule l'autorisation. Si le positionnement bloque, demander une manœuvre au Pilote ; les réglages du poste Armement ne déplacent pas votre sous-marin.

### Étape 6 — Tirer

1. Appuyer sur **Tirer…**.
2. Vérifier la sélection, puis appuyer sur **CONFIRMER TIR**.

La torpille part alors à **40 nœuds**, sur un **cap fixe**, pour une course maximale de **8 km**. Elle n'est pas guidée après lancement : un changement de route de la cible peut la faire manquer.

La solution indique un cap de tir, un temps d'interception estimé et une incertitude spatiale. Les avertissements signalent notamment une piste peu fiable, une distance ou un relèvement incertains, une profondeur incompatible ou un secteur de tube incorrect.

### Étape 7 — Remettre le tube en état

Après lancement, la porte se ferme automatiquement en **3 secondes**. Appuyer ensuite sur **Purger** et attendre **8 secondes**, puis recharger si le stock le permet.

Pour abandonner une préparation avant le tir, utiliser **Fermer / purger** : d'abord fermer si le tube est ouvert, attendre la fin de fermeture, puis purger. Une torpille encore chargée est conservée.

## 5. Jouer au poste Capitaine avec un bot Armement

1. Sélectionner la piste à attaquer.
2. Appuyer sur **Ordre bot**. Le bot choisit un tube disponible et prépare une attaque avec les réglages par défaut.
3. Parcourir les tubes pour retrouver celui que le bot prépare : le tube choisi par le bot n'est pas nécessairement celui actuellement affiché.
4. Vérifier sa piste configurée et sa solution, puis appuyer sur **Autoriser**.
5. Le bot lance lorsqu'il est prêt et que les conditions de lancement sont réunies, puis effectue la remise en état.

Si un humain occupe l'Armement, **Ordre bot** est refusé : transmettre la cible au joueur, qui configure et prépare son tube. Le Capitaine reste responsable de l'autorisation, l'Armement du lancement.

## 6. Comprendre les blocages

| Message ou symptôme | Ce qu'il faut vérifier |
|---|---|
| `Le tube … vise P…, piste perdue ou non partagee` | Choisir une piste encore partagée puis **Configurer**. Si vous êtes Capitaine, demander la reconfiguration à l'Armement ou donner un nouvel **Ordre bot**. |
| `SELECTION NON APPLIQUEE` | Le tube vise toujours une autre piste. **Configurer** applique le nouveau choix. |
| `REGLAGES NON APPLIQUES` | Les corrections locales diffèrent de celles du tube. Appliquer ou rétablir les réglages avant le tir. |
| `Distance cible inconnue`, `Cap cible inconnu`, `Vitesse cible inconnue` | Attendre que les observations du Sonar permettent d'estimer la donnée manquante. |
| `Vitesse cible corrigee egale a celle de la torpille` | Limitation actuelle du calcul : attendre une nouvelle estimation ou ajuster la correction de vitesse puis **Configurer**. |
| `Aucune interception avec ces estimations` | Attendre de nouvelles observations ou corriger les réglages. |
| `Interception au-dela de l'autonomie` | Rapprocher le sous-marin ou choisir une autre piste. |
| `Direction du tir hors secteur` | Faire manœuvrer le Pilote ou utiliser un tube avant/arrière adapté à la direction du tir. |
| `EN COURS` | Une opération temporisée est en cours. Le message indique le temps restant ; attendre avant la suite. |
| `AUTORISATION MANQUANTE` | Le Capitaine doit autoriser la configuration du tube. |
| `PROCEDURE : ETAPE DU TUBE INCOMPATIBLE` | Une étape manque ou une opération est encore en cours. Attendre le statut final avant de poursuivre. |
| `ATTENDRE L'AUTORISATION DU CAPITAINE` | Obtenir une autorisation sur la configuration actuelle du tube. |
| `PISTE EXPIREE OU NON PARTAGEE` | Demander au Sonar une piste encore présente et partagée, puis reconfigurer le tube. |
| `RESERVE VIDE` | Vérifier les autres tubes : des torpilles peuvent y être déjà chargées. |
| Bouton sombre | L'action paraît indisponible dans l'état actuel. L'interface ne détaille pas encore systématiquement la raison près du bouton. |

### Cas d'une ancienne capture : tube chargé, piste à 40 nœuds

Les premières captures utilisaient les libellés `Config. piste None` / `Some(…)` et un message générique d'interception. Ils ont été remplacés par les états explicites ci-dessus.

Sur l'exemple `P6 7 deg / 3878m / conf. 69%`, `Cible cap 104 / 40.0 kn`, `TUBE 4 AV / Charge`, `Config. piste None` :

- P6 est sélectionnée, mais **aucune configuration n'a été acceptée** pour le tube 4.
- `Prof. 3m` correspond au réglage local de la torpille ; il n'est pas encore appliqué au tube.
- La vitesse de 40 nœuds est une estimation sonar, qui peut être instable.
- **Limitation du prototype :** le calcul actuel refuse le cas où la vitesse cible corrigée est exactement égale à celle de la torpille, même si certaines géométries pourraient permettre une interception. Ce n'est donc pas forcément une erreur de procédure du joueur.

Pour poursuivre : sélectionner par exemple une piste `Merchant` partagée, vérifier ses estimations, conserver une profondeur de torpille de 4 m et tenter **Configurer**. Si l'estimation de vitesse reste à 40 nœuds, attendre d'autres observations ou essayer une correction manuelle de vitesse avant de reconfigurer. Une fois la configuration acceptée : **Inonder → Ouvrir → autorisation → Tirer… → CONFIRMER TIR**.

`Derniere commande refusee` décrit un ancien résultat de commande, pas nécessairement le blocage actuel. Il est effacé à la prochaine commande acceptée. `Derniere action locale` décrit votre dernier appui ; ce n'est pas une confirmation serveur. Les messages de blocage et de prochaine étape sont calculés depuis le snapshot courant. `CONFIGURATION APPLIQUEE`, l'état du tube et la solution permettent de vérifier le résultat effectif.

## 7. Savoir si le tir a touché

- `courses suivies` compte les courses propres suivies, pas les ennemis touchés.
- Une **explosion observée** signale un impact entendu à portée d'écoute (6 km dans cette version). Elle ne confirme ni l'identité ni l'intégrité restante d'une cible.
- Une **fin de course** correspond à l'épuisement estimé de l'autonomie. Ce n'est pas une preuve absolue que le tir a manqué : un impact hors portée d'écoute reste caché et sa course estimée continue jusqu'à son terme.
- La trajectoire peut passer trop loin, devenir fausse après une manœuvre adverse, ou passer sous une coque si la profondeur réglée dépasse 8 m.

Pour les règles détaillées et les fonctionnalités à venir, voir le [GDD](game-design.md) et la [roadmap](roadmap.md).
