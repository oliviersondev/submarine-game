# Recette M5.3 — Contrôle des avaries

## Préparer

1. Redémarrer `make server`, lancer `make client`, recharger tous les onglets : protocole v10 incompatible v9/v8.
2. Ouvrir une salle avec Pilote, Sonar, Capitaine et Ingénierie humains. Garder le Capitaine sans autorisation de tir pour ne pas couler l'escorte ; les rôles vacants sont tenus par des bots.
3. Tous prêts, démarrer. Vérifier coque 100%, eau/feu 0%, cinq systèmes OK, équipe disponible, aucune pompe ni isolation.
4. Refaire l'approche de [M5.2](recette-m5.2.md) : vers le nord, 8 nœuds et 100 m, Sonar émettant périodiquement pour provoquer la recherche. Ralentir au contact pour subir une passe, puis tenter de rompre le contact. L'approche peut prendre plusieurs minutes et n'assure pas un coup proche ; aucune avarie de recette n'est injectée par une commande de jeu.

## Une passe et la décision de priorité

- Après un coup proche, noter la coque, le compartiment atteint, la fuite, l'eau, le feu et les systèmes **EN PANNE**. Un coup de moins de 8 points peut ne provoquer ni feu ni panne : attendre un autre coup si nécessaire, sans rester sous le feu jusqu'à la destruction.
- Vérifier que les autres postes reçoivent la synthèse feu/systèmes indisponibles, pas le détail de l'équipe ou des compartiments.
- En Ingénierie, défiler jusqu'aux commandes et sélectionner AVANT, CENTRAL ou MACHINES. Vérifier que chaque action cible ce compartiment.

## Isoler, pomper, éteindre, colmater

1. **ISOLER** le compartiment atteint : l'eau et le feu n'en sortent plus vers les voisins ; ses équipements deviennent indisponibles. L'eau peut encore monter par la fuite.
2. **POMPE DEMARRER** : l'eau doit baisser si la pompe surpasse l'entrée d'eau. La charge électrique et le bruit augmentent. Sans colmatage, arrêter la pompe laisse la fuite agir à nouveau.
3. **ETEINDRE** : l'équipe travaille 18 s ; le feu ne disparaît pas instantanément. Un feu peut encore provoquer des dégâts pendant l'intervention.
4. Pendant le travail, demander **COLMATER** : refus explicite « équipe occupée ». **ANNULER INTERVENTION**, puis relancer l'extinction : le délai repart du début, sans équipe supplémentaire.
5. Une fois le feu maîtrisé, **COLMATER** : après 20 s, la fuite passe à zéro, mais l'eau présente et les dégâts de coque restent.
6. Laisser pomper jusqu'à assèchement, puis arrêter : l'eau ne doit plus remonter sans nouvelle avarie.

Après extinction, si aucun système n'est indisponible, la synthèse affiche **AUCUN INCENDIE ACTIF**, même lorsque les derniers dégâts ont été causés par un incendie. Les anciens faits restent dans la section **DERNIERS DEGATS (HISTORIQUE)**, avec leur tick : ils ne signalent pas un feu actuel. Un feu strictement positif inférieur à 1% s'affiche **FEU <1%** dans la synthèse et le compartiment, jamais **FEU 0%** par arrondi.

Pendant **ETEINDRE**, **COLMATER** ou **REPARER**, vérifier que seul le bouton de l'intervention active du compartiment sélectionné est vert et se remplit de gauche à droite. Le libellé affiche le pourcentage accompli : 13,5 s restantes sur 18 s correspondent à **25%**, et non 75%. Changer de compartiment ne doit pas activer un bouton homonyme ; revenir sur le compartiment de travail réaffiche sa progression. Pour les réparations, seul le système effectivement traité est mis en évidence. Annulation, fin de l'intervention et perte du sous-marin suppriment cette indication d'activité.

Si l'intervention devient dangereuse après un nouvel impact, vérifier « SUSPENDU » et le temps restant immobile. Pomper ou changer de priorité pour rétablir des conditions sûres. Le colmatage exige feu <25% et eau <90% ; la réparation de système feu <25% et eau <50%.

Le bouton suspendu est orange, porte aussi **SUSPENDU** et conserve le même remplissage tant que le serveur ne fait pas avancer le travail. La reprise revient au vert sans remettre à zéro la progression.

## Réparer et reprendre la navigation

- Utiliser **REPARER …** pour chaque système réellement en panne du compartiment sélectionné : 25 s par système, avec une seule équipe.
- Un système réparé reste indisponible si son compartiment est isolé, trop inondé ou trop en feu. La liste distingue **EN PANNE** de **ISOLE / FEU / EAU**.
- **ROUVRIR** quand les risques sont maîtrisés, puis remettre moteurs/diesels/ventilation/recharge selon la situation ; la panne avait coupé les équipements, elle ne doit pas avoir changé les consignes Pilote.
- Vérifier qu'une commande de système indisponible reçoit un refus précis ; le retour en service autorise de nouveau cette commande. Les tubes sont à l'avant, sonar/gouvernes au central, moteurs/ventilation aux machines.
- Vérifier que la coque et l'intégrité structurelle ne sont pas régénérées. La remontée d'urgence reste possible avec les gouvernes indisponibles, sous réserve de l'inondation.

## Propagation et énergie — contrôles complémentaires

- Lorsqu'un compartiment non isolé dépasse 50% d'eau ou 60% de feu, observer uniquement la propagation progressive aux voisins. Fermer la porte doit arrêter ces échanges, sans annuler l'avarie source. Ce scénario peut être difficile à atteindre avant la perte de coque ; les tests déterministes couvrent directement ces seuils.
- Une pompe sans batterie doit être refusée au démarrage. Si la batterie s'épuise après activation, l'état indique « SANS COURANT » : pas d'eau évacuée, pas de bruit de pompe, demande de pompage conservée.
- Ne pas chercher à réparer un sous-marin déjà perdu : bandeau rouge permanent et commandes refusées.

## Vérifier le bot, dans une nouvelle salle

- Refaire la préparation mais **laisser l'Ingénierie au bot**, avec Pilote/Sonar/Capitaine humains.
- Après un coup, rompre le contact. Observer feu et eau diminuer, les systèmes revenir et la navigation redevenir possible, avec des dégâts de coque persistants.
- Le bot ne doit ni déplacer les consignes Pilote, ni tout réparer instantanément. Il ne garantit pas la survie sous plusieurs passes successives ou sans énergie. Les tests automatiques vérifient le détail des commandes et des délais, que les postes humains ne voient pas tous.

## Interface et retour

Tester 1280×720, 360×640 et 640×360 : défilement molette/glissement, boutons d'au moins 44 px, sélection de compartiment lisible, état d'équipe et erreurs accessibles, aucun chevauchement avec les bandeaux d'explosion/perte. Tester en particulier une erreur pendant une avarie, pas seulement un écran sain.

Noter scénario, rôle, tick/temps, compartiment, action et résultat ; joindre une capture pour les problèmes d'affichage. L'équilibrage et la manipulation sur téléphone réel restent à confirmer.
