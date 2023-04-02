import subprocess

CHROME_BIN = "/snap/bin/chromium"
WEBPAGE_TO_SKP_BIN = "./experimental/tools/web_to_skp"
SKP_DIR = "./skia_opt_research/webpage_skps/"
WAIT_BEFORE_RENDER = "5"

wp_to_url = {
    "amazon": "https://www.amazon.com",
    "google": "https://www.google.com",
    "wikipedia": "https://en.wikipedia.org",
    "booking" : 'http://www.booking.com/searchresults.html?src=searchresults&latitude=65.0500&longitude=25.4667',
    "utah_soc" : "https://cs.utah.edu",
    "ebay": "http://www.ebay.com",
    "volkswagen": "https://www.capitolvolkswagen.com/new-vehicles/",
    "cnn": "http://www.cnn.com",
    "linkedin": "https://www.linkedin.com/in/linustorvalds",
    "twitter": "https://twitter.com/katyperry",
    "chalkboard": 'https://testdrive-archive.azurewebsites.net/performance/chalkboard/Images/Chalkboard.svg',
    "carsvg": 'http://codinginparadise.org/projects/svgweb/samples/svg-files/car.svg',
    "gujarati_wiki": 'https://en.wikipedia.org/wiki/Gujarati_phonology',
    "the_verge": 'http://theverge.com/'
}

for webpage in wp_to_url:
    skp_dir = SKP_DIR + webpage
    skp_command = ([WEBPAGE_TO_SKP_BIN, CHROME_BIN, wp_to_url[webpage], skp_dir, WAIT_BEFORE_RENDER])
    print(skp_command)
    subprocess.run(skp_command)
    copy_command = "for f in " + skp_dir + "/*.skp; do cp -- \"$f\" \""+skp_dir+ "_$(basename $f)\"; done"
    subprocess.run(copy_command, shell=True)
    subprocess.run("rm -r "+skp_dir, shell=True)
