import sys
import csv
import os
import jinja2
import argparse

'''
This script assumes that skp_opt_membench will dump the following files in a directory
1. output a 000_summary_csv.txt
2. For each skp in the above file, a <skp_name>_<optimization>_log.txt file which contains memory allocations per draw call.

The script will write an index.html in the report directory.

Usage
python gen_report.py -d <dir_from_membench>
'''

parser = argparse.ArgumentParser(description='Generate a HTML report of skp_opt_membench results.')
parser.add_argument('-d', '--report_dir', help='directory containing results of a skp_opt_membench run')
parser.add_argument('-t', '--report_template', default='report_template.html', help='path to the html template')

CSV_SUMMARY_FILE_NAME = "000_summary_csv.txt"

args = parser.parse_args()

membench_summary_filepath = os.path.join(args.report_dir, CSV_SUMMARY_FILE_NAME);
report_template_filepath = os.path.abspath(args.report_template)

with open(membench_summary_filepath) as csvfile:
    results_csv = csv.DictReader(csvfile)
    skp_name_field = results_csv.fieldnames[0]
    opts = results_csv.fieldnames[1:]

    skp_membench_results = []
    for result_csv_row in results_csv:
        skp_membench_result = {}
        skp_membench_result['name'] = os.path.basename(result_csv_row[skp_name_field])
        skp_membench_result['ref_img_url'] = ("renders/%s.png" % os.path.basename(result_csv_row[skp_name_field]))
        for opt in opts:
            skp_membench_result[opt] = {}
            skp_membench_result[opt]['value'] = result_csv_row[opt]
            skp_membench_result[opt]['link'] = ('./%s_%s_log.txt' % (skp_membench_result['name'], opt)) 

            # Error Handling - Negative numbers indicate an error.
            # TODO(chesetti): Add some documentation in skia_opt_membench.cpp about error types.
            # Also see if there's a better way to have these error codes synced across the bench and report generator.
            # Consider using string values instead of negative numbers.
            if skp_membench_result[opt]['value'] == '-1':
                skp_membench_result[opt]['value'] = 'SkiOpt had trouble parsing this.'
                skp_membench_result[opt]['link'] = ('./%s.json.error_log.txt' % (skp_membench_result['name'])) 

            if skp_membench_result[opt]['value'] == '-2':
                skp_membench_result[opt]['value'] = 'SkiOpt optimization resulted in image diffs.'
                # TODO(chesetti): Make the link point to the image diff.


        skp_membench_results.append(skp_membench_result)

    template_loader = jinja2.FileSystemLoader(searchpath = "/")
    template_env = jinja2.Environment( loader=template_loader)

    template = template_env.get_template(report_template_filepath)
    template_vars = {
        "title": os.path.basename(args.report_dir),
        "skp_membench_opts": opts,
        "skp_membench_results": skp_membench_results
    }

    with open(os.path.join(args.report_dir, "index.html"), "w") as f:
        f.write(template.render(template_vars))