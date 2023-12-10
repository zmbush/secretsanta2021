import csv
import re
import sys
import random
from typing import DefaultDict

CANONICAL_NAMES = {
    "boa": "boa hancock",
    "bon": "bon clay",
    "borsalino": "kizaru",
    "buggy the clown": "buggy",
    "caesar clown": "caesar",
    "charlotte smoothie": "smoothie",
    "corazón": "corazon",
    "donquixote doflamingo": "doflamingo",
    "donquixote rosinante": "corazon",
    "eustass kid": "kidd",
    "femluffy": "luffyko",
    "gecko moria": "moria",
    "geko moria": "moria",
    "god enel": "enel",
    "issho": "fujitora",
    "jack the drought": "jack",
    "jimbe": "jinbe",
    "jimbei": "jinbe",
    "jinbei": "jinbe",
    "kid": "kidd",
    "koby": "coby",
    "kuzan": "aokiji",
    "monkey d. luffy": "luffy",
    "monkey d. garp": "garp",
    "monkey d garp": "garp",
    "monkey d luffy": "luffy",
    "monkey d. dragon": "dragon",
    "monkey d dragon": "dragon",
    "marco the phoenix": "marco",
    "marshall d. teach": "blackbeard",
    "marshall d teach": "blackbeard",
    "nico robin": "robin",
    "otama": "tama",
    "portgas d.ace": "ace",
    "portgas d. ace": "ace",
    "portgas d ace": "ace",
    "queen the plague": "queen",
    "rob lucci": "lucci",
    "rocinante": "corazon",
    "rocks d xebec": "rocks",
    "roronoa zoro": "zoro",
    "rosinante": "corazon",
    "sakazuki": "akainu",
    "señor pink": "senor pink",
    "sir crocodile": "crocodile",
    "smo": "smoker",
    "straw hats": "strawhats",
    "tony tony chopper": "chopper",
    "tonytony chopper": "chopper",
    "trafalgar d water law": "law",
    "trafalgar d. water law": "law",
    "trafalgar law": "law",
    "trafalger d. water law": "law",
    "tralfagar law": "law",
    "trafalgar law 💛": "law",
    "vander decken": "van der decken",
    "vinsmoke sanji": "sanji",
    "x drake": "drake",
    "xdrake": "drake",
    # Ships
    "zosan": "zoro/sanji",
    "frobin": "franky/robin",
    "lawlu": "law/luffy",
    "sanuso": "sanji/usopp",
    "kidkiller": "kidd/killer",
    "zolu": "zoro/luffy",
    "zorobin": "zoro/robin",
    "sabala": "sabo/koala",
    "namivivi": "nami/vivi",
    "acesabo": "ace/sabo",
    "asl brothers": "ace/sabo/luffy",
    "asl": "ace/sabo/luffy",
    "dofladile": "doflamingo/crocodile",
    "zolaw": "zoro/law",
    "zolalwlu": "zoro/law/luffy",
    "kiddxlaw": "kidd/law",
    "deucexace": "deuce/ace",
    "sanji and nami": "sanji/nami",
    "zoro and chopper": "zoro/chopper",
    "sanji + zoro": "zoro/sanji",
    "doflaw": "doflamingo/law",
    "doflacora": "doflamingo/law/corazon",
    "lawnami": "law/nami",
    "yamatoace": "yamato/ace",
    "peronazoro": "perona/zoro",
    "kidlu": "kidd/luffy",
    "kidlawlu": "kidd/law/luffy",
    "yamakiku": "yamato/kiku",
    "izodenjiro": "izo/denjiro",
    "izothatch": "izo/thatch",
    "lawkins": "law/hawkins",
    "luvi": "luffy/vivi",
    "sanami": "sanji/nami",
    "zonami": "zoro/nami",
    "zolu": "zoro/luffy",
    "zosan": "zoro/sanji",
    "frobin": "franky/robin",
    "zotash": "zoro/tashigi",
    "zolusan": "zoro/luffy/sanji",
    "navi": "nami/vivi",
    "lulu": "lucci/luffy",
    "sakaissho": "sakazuki/issho",
    "sakabors": "sakazuki/borsalino",
    "zorotashigi": "zoro/tashigi",
    "luffyxsanji": "luffy/sanji",
    "smokertashigi": "smoker/tashigi",
    "rogerb": "roger/OC",
    "rayb": "rayleigh/OC",
    "barco": "marco/OC",
    "zolawlu": "zoro/law/luffy",
    "lzs": "luffy/zoro/sanji",
    "brookyorki": "brook/yorki",
    "kayausopp": "kaya/usopp",
    "lawbin": "law/robin",
    "marlaw": "marco/law",
    "zorosanji": "zoro/sanji",
    "namitashigi": "nami/tashigi",
    "namiwanda": "nami/wanda",
    "garproger": "garp/roger",
    "sabokoala": "sabu/koala",
    "luccipaulie": "lucci/pauli",
    "yamaace": "yama/ace",
    "lawluffy": "law/luffy",
    "lucciluffy": "luccy/luffy",
    "acelu": "ace/luffy",
    "sabolu": "sabo/luffy",
    "kidlaw": "kidd/law",
    "boalu": "boa hancock/luffy",
    # other
    "none": None,
    "n/a": None,
    "//": None,
}

FAV_CHAR = "Favorite character(s)"
HATE_CHAR = "Characters I do not want to create for"
FAV_SHIPS = "Favorite ships"
HATE_SHIPS = "Ships I do not want to create for"

# FIRST = "chloensolomon@gmail.com"
# SECOND = "mushroomgrenadework@gmail.com"
# MUST_MAKE = {"chloensolomon@gmail.com": "mushroomgrenadework@gmail.com"}


def isIncompatible(creator, giftee):
    favs = set(process(FAV_CHAR, giftee))
    hates = set(process(HATE_CHAR, creator))
    if len(favs - hates) < len(favs):
        return True

    favs = set(process(FAV_SHIPS, giftee))
    hates = set(process(HATE_SHIPS, creator))
    if len(favs - hates) < len(favs):
        return True

    # if creator["Email Address"] in MUST_MAKE:
    #     if MUST_MAKE[creator["Email Address"]] != giftee["Email Address"]:
    #         raise "SHIT"
    #         return True

    return False


def process(columnName, row):
    results = []
    for char in (
        re.sub(r"\(.*\)", "", row[columnName])
        .replace("-", ",")
        .replace("?", "")
        .replace("!", "")
        .replace(" x ", "/")
        .replace("&", "/")
        .replace(" / ", "/")
        .split(",")
    ):
        v = char.strip().lower()
        if v:
            if v in CANONICAL_NAMES:
                v = CANONICAL_NAMES[v]
            if not v:
                continue
            if "/" in v:
                v = "/".join(
                    sorted(
                        CANONICAL_NAMES[n] if n in CANONICAL_NAMES else n
                        for n in v.split("/")
                    )
                ).strip()
            results.append(v)

    return results


with open(sys.argv[1], encoding="utf-8") as csvfile:
    path = 3
    if path == 0:
        ships = DefaultDict(int)
        for row in csv.DictReader(csvfile):
            for ship in process(FAV_SHIPS, row):
                ships[ship] += 1
            for ship in process(HATE_SHIPS, row):
                ships[ship] += 1

        for row in sorted(ships.items(), key=lambda item: item[1]):
            print(row[0], row[1])
    elif path == 1:
        chars = DefaultDict(int)
        for row in csv.DictReader(csvfile):
            for char in process(FAV_CHAR, row):
                chars[char] += 1
            for char in process(HATE_CHAR, row):
                chars[char] += 1

        for row in sorted(chars.items(), key=lambda item: item[1]):
            print(row[0], row[1])
    elif path == 2:
        email = DefaultDict(int)
        for row in csv.DictReader(csvfile):
            email[row["Email Address"]] += 1

        for row in email.items():
            if row[1] > 1:
                print(row[0], row[1])
    else:
        allRows = [row for row in csv.DictReader(csvfile)]
        reader = [
            row
            for row in allRows
            # if row["Email Address"] != FIRST and row["Email Address"] != SECOND
        ]
        # first = [row for row in allRows if row["Email Address"] == FIRST][0]
        # second = [row for row in allRows if row["Email Address"] == SECOND][0]
        for j in range(0, 10000000):
            if j % 100 == 0:
                print(f"Round {j}")
            random.shuffle(reader)
            # fullList = [first, second] + reader
            fullList = reader
            good = True
            for (i, creator) in enumerate(fullList):
                giftee = fullList[(i + 1) % len(fullList)]
                if isIncompatible(creator, giftee):
                    good = False
                    break
            if good:
                print(f"Found valid assignments after {j+1} rounds")
                for (i, creator) in enumerate(fullList):
                    print(f"{creator['Email Address']}")
                break
