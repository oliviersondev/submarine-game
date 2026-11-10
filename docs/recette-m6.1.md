# Recette M6.1 — Mission niveau 1

## Préparation

- Vérifier `make check-all`, `make test` et `cargo check -p client --target wasm32-unknown-unknown --tests`.
- Redémarrer le serveur après mise à jour : protocole **v14**, incompatible v13. Lancer `make server` et `make client` dans deux terminaux.
- Tester navigateur ordinateur et téléphone réel, notamment 360 × 640. Lancer en Capitaine (`?role=captain`), créer une salle ; les quatre autres postes seront bots.
- Les étapes de recette ci-dessous sont une procédure ; seuls les contrôles explicitement listés dans la vérification partielle ont été réalisés.

### Vérification partielle du 10 novembre 2026

- `make test` : 152 tests natifs réussis (28 serveur, 12 shared, 112 simulation), dont un scénario de tir visuel avec bot Armement puis navigation vers l'extraction, rejoué à l'identique dans une scène contrôlée.
- `make check-all` et compilation des tests client pour WASM réussis. Les tests client ne sont pas exécutés par `make test`.
- Chrome headless/WebGL2 avec serveur de test v14 séparé : briefing aux deux profils, fermeture/réouverture, défilement à la molette, prêt/démarrage, phase attaque et repère d'extraction vérifiés à 360 × 640 ; carte également vérifiée à 1280 × 720. Pas d'erreur d'exécution JavaScript/WASM observée. Les boutons de mission disposent d'une zone réservée hors télémétrie.
- Correction des proportions du briefing : les deux profils ont été revérifiés dans Chrome à 360 × 640, 640 × 360 et 1885 × 720, puis au retour en portrait. Le dessin conserve un rapport 140:24, reste centré et limite sa largeur à 350 px, sans étirement du navire. Vérifications WASM et 152 tests natifs toujours réussis ; le nouveau test de configuration du dessin est compilé pour WASM, pas exécuté par `make test`.
- Correction du recouvrement du bouton **PRET** sur grand écran : les commandes du lobby commencent sous la ligne de briefing. Visibilité des boutons et retour après redimensionnement vérifiés à 1885 × 720, 360 × 640 et 640 × 360 ; un clic sur **PRET** donne bien « HUMAIN PRET ». Compilation WASM, y compris du test de non-recouvrement, réussie.
- Carte nautique fictive : grille et coordonnées, cercle d'extraction rond, boutons de cadrage/zoom/recentrage, zoom à la molette et glissement souris vérifiés dans Chrome à 1280 × 900. Sous-marin orienté vérifié pendant une manœuvre réelle. Formats 360 × 640 et 640 × 360 vérifiés ; glissement tactile et bouton **EXTRACTION** également essayés par événements tactiles émulés. Pas d'exception JavaScript/WASM observée. Quatre tests de géométrie de `chart.rs` exécutés dans un harness temporaire Bevy minimal sans audio : projection uniforme/inverse, limites du zoom/déplacement, coordonnées et découpage. La compilation des tests client reste distincte de leur exécution complète. Course de torpille sur la nouvelle carte, poste Armement humain et téléphone physique restent à recetter.
- Non validés par cette vérification : mission complète dans le navigateur, résultat après combat réel, cinq humains, toucher sur téléphone physique, lisibilité des navires à distance réelle et équilibrage normal. Les étapes ci-dessous restent la recette complète à effectuer.

## Briefing et reconnaissance

1. Vérifier l'ouverture du briefing au lobby, sa description et le profil dessiné, la composition « 2 cargos + 1 escorte », l'extraction X 0 / Y −2 000 m / rayon 500 m et la limite ±7 000 m.
2. Fermer, se déclarer prêt et démarrer. Rouvrir avec **MISSION / BRIEFING**. Vérifier fermeture, défilement tactile/molette et accès au dernier bouton sans clavier.
   Briefing fermé, vérifier sur grand écran que **PRET** et **DEMARRER** sont visibles et cliquables sous **MISSION / BRIEFING**, sans recouvrement. Refaire après passage portrait/paysage et retour au grand écran.
   Redimensionner en portrait, paysage et grand écran : le profil doit rester centré et conserver ses proportions (140:24), sans s'étirer avec la largeur du bandeau. Vérifier les deux cargos ; seule la taille uniforme du dessin peut changer.
3. Au périscope, observer les deux cargos : château arrière/une cheminée/deux grands mâts contre château central/deux cheminées/mâts courts. Comparer au dessin du briefing et sous plusieurs angles. Il est normal qu'ils soient difficiles à identifier de face ou de très loin ; noter la distance de reconnaissance sur téléphone.
4. Désigner volontairement le mauvais cargo : aucun indicateur ne doit valider ou invalider l'identité de l'objectif. Les accusés de commande peuvent confirmer l'application des hypothèses, pas leur justesse.

## Objectif et extraction

1. Suivre le [guide armement](guide-armement.md), préparer et autoriser le tir contre la cible du briefing.
2. Après sa destruction, vérifier l'annonce immédiate **OBJECTIF DETRUIT / REJOIGNEZ L'EXTRACTION**. L'annonce est permise même périscope abaissé et hors écoute ; elle ne corrige pas les pistes ou les courses estimées.
3. Fermer/réouvrir le briefing : la phase extraction doit rester présente. Vérifier le repère vert, distance et rayon sur la carte Capitaine/Armement.
4. Ordonner au bot Pilote une route vers X 0 / Y −2 000 m, sans dépasser la zone. Entrer dans le rayon de 500 m : victoire même si l'escorte poursuit encore.
5. Vérifier écran de victoire, durée, coque restante, physique figée et refus des nouvelles commandes. Fermer/réouvrir le panneau ne relance pas la partie.

## Carte nautique fictive

1. Fermer le briefing et le poste de commande. Vérifier la carte nord en haut, les graduations latitude/longitude, la mention **SECTEUR FICTIF**, les coordonnées propres et une barre d'échelle en MN/mètres. Au départ X/Y = 0/0 correspond à 48°00.00' N / 10°00.00' O, affiché `48d00.00'N / 10d00.00'O` (`d` = degrés, pour compatibilité avec la police du client) ; ce n'est pas une zone géographique réelle à reconnaître.
2. Vérifier la présence du sous-marin bleu, vu de dessus, avec une ligne vers l'avant. Ordonner un changement de cap au Pilote : l'icône doit suivre le cap réel, pas sa consigne instantanément. Sa taille reste symbolique et lisible.
3. Vérifier le cercle vert d'extraction et son rayon de 500 m. Appuyer sur **EXTRACTION** pour le cadrer : un vrai cercle doit apparaître, pas une ellipse sur un écran large. La distance reste calculée depuis le sous-marin, même en vue libre. Le carré d'opération est une limite publique, pas une détection ennemie.
4. Tester **+**, **−**, et la molette au-dessus de la carte. Le cercle, la grille, les pistes et les courses changent ensemble d'échelle. Aux limites du zoom, l'affichage reste fini et stable. Sur téléphone, utiliser les boutons (pas de pincement requis).
5. Glisser la carte avec la souris, puis au toucher : la mention **VUE LIBRE** apparaît. Le sous-marin ne reçoit aucun ordre de navigation. Continuer à naviguer : la carte ne doit pas se recoller au sous-marin. **RECENTRER** le replace au centre et réactive le suivi, en conservant le zoom.
6. Panoramique/zoom pendant une course de torpille : pistes, trajet prévu, interception supposée et torpilles restent dans le même repère. Un objet hors champ disparaît plutôt que d'être collé à un bord. Les pistes ont la mention **EST.** ; un relèvement sans distance porte **DIST. ?**, sans position inventée.
7. Passer de 360 × 640 à 640 × 360 puis grand écran et revenir. Vérifier cercle toujours rond, échelle cohérente, boutons accessibles, grille non étirée et position géographique stable du centre en vue libre. Les zooms ne doivent pas agir derrière le briefing ou le panneau périscope, ni lorsque la molette est au-dessus des commandes d'armement.
8. En multijoueur, vérifier la carte au poste Armement et l'absence de régression de la vue polaire Sonar. Ces manipulations restent locales au client ; elles ne changent ni les estimations reçues ni la mission.

## Erreurs et défaites (nouvelles salles)

- Couler l'autre cargo : aucun accomplissement d'objectif, aucune confirmation automatique de ce naufrage. Les perceptions d'explosion restent possibles. Rejoindre l'extraction ne donne pas la victoire.
- Entrer dans l'extraction avant d'avoir détruit la cible : cercle ambre et message **ZONE D'EXTRACTION ATTEINTE / OBJECTIF PRINCIPAL NON ACCOMPLI / MISSION TOUJOURS EN COURS** sous la carte, avant la liste des pistes. Le message persiste à l'arrêt et en vue libre, sans bloquer les commandes ; sortir du rayon de 500 m le fait disparaître et remet le cercle en vert. Aucun écran de résultat n'apparaît. Après accomplissement de l'objectif ou fin de mission, ce message ne doit pas contredire le nouvel état.
- Laisser le cargo prioritaire vivant sortir du carré de ±7 000 m : **DEFAITE / CARGO CIBLE ECHAPPE**, état figé. Les cargos ne font plus demi-tour sur la frontière.
- Provoquer la perte du sous-marin avant ou après destruction de la cible : **DEFAITE / SOUS-MARIN PERDU**, état figé, commandes refusées.
- Recharger la page puis créer une nouvelle salle : nouveau briefing correspondant à la nouvelle graine. Aucune relance dans la même salle n'est promise.

## Multijoueur et informations

- Vérifier briefing et résultat aux cinq postes, conservation des commandes de chacun, absence d'ordre bot sur un poste humain.
- Aucun profil ne porte de badge « bonne cible » au périscope ; aucune position ennemie exacte, intégrité, résultat des autres naufrages ou état de poursuite n'est ajouté aux snapshots.
- L'annonce d'objectif doit rester compréhensible sans son. Équilibrage en configuration normale et distinction des profils sur téléphone réel restent à confirmer.
