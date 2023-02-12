import sys
import csv
import os
import jinja2
import argparse
import skia_opt_metrics_pb2 as SkiaOptMetrics

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

PROTO_SUMMARY_FILE = "benchmark.pb"

args = parser.parse_args()

proto_summary_filepath = os.path.join(args.report_dir, PROTO_SUMMARY_FILE);
report_template_filepath = os.path.abspath(args.report_template)

proto_file = open(proto_summary_filepath, "rb")
proto_data = proto_file.read()
proto_file.close()
benchmark = SkiaOptMetrics.SkiaOptBenchmark()
benchmark.ParseFromString(proto_data)

opts = ['NO_OPT', 'SKIA_RECORD_OPTS', 'SKIA_RECORD_OPTS_2']
skp_membench_results = []

summary_result = {}
summary_result['name'] = 'SUMMARY'
summary_result['json'] = ('#')
summary_result['ref_img_url'] = ('#')
summary_result['skipass_log'] = ('#')
summary_result['skipass_value'] = 0
summary_result['skipass_link'] = '#'
for opt in opts:
    summary_result[opt] = {}
    summary_result[opt]['value'] = 0
    summary_result[opt]['link'] = '#'

for skp_benchmark in benchmark.skp_benchmark_runs:
    skp_name = os.path.basename(skp_benchmark.skp_name)

    skp_membench_result = {}
    skp_membench_result['name'] = skp_name
    skp_membench_result['json'] = ('json/%s.json' % skp_name)
    skp_membench_result['ref_img_url'] = ("renders/%s.png" % skp_name)
    skp_membench_result['skipass_log'] = ('./%s_SKI_PASS_SkiPassRunResult.txt' % (skp_name))

    # First extract SKI_PASS results.
    for opt_benchmark in skp_benchmark.optimization_benchmark_runs:
        opt = SkiaOptMetrics.Optimization.Name(opt_benchmark.optimization_type)
        if opt != 'SKI_PASS':
            continue
        skp_membench_result['skipass_value'] = opt_benchmark.malloc_allocated_bytes
        skp_membench_result['skipass_link'] = ('./%s_%s_log.txt' % (skp_membench_result['name'], opt)) 

        skipass_mem = int(opt_benchmark.malloc_allocated_bytes)
        summary_result['skipass_value'] += skipass_mem

    for opt_benchmark in skp_benchmark.optimization_benchmark_runs:
        opt = SkiaOptMetrics.Optimization.Name(opt_benchmark.optimization_type)
        if opt == 'SKI_PASS':
            continue
        skp_membench_result[opt] = {}
        skp_membench_result[opt]['value'] = opt_benchmark.malloc_allocated_bytes
        skp_membench_result[opt]['link'] = ('./%s_%s_log.txt' % (skp_membench_result['name'], opt)) 

        mem = int(opt_benchmark.malloc_allocated_bytes)
        if mem != 0: 
            skp_membench_result[opt]['comp'] = "{:.1f}".format(100.0 * (mem-skipass_mem)/mem)
        elif skipass_mem == 0:
            skp_membench_result[opt]['comp'] = 0.0
        else:
            skp_membench_result[opt]['comp'] = 'WTF, we made it worse!'
        summary_result[opt]['value'] += mem


    skp_membench_results.append(skp_membench_result)

for opt in opts:
    mem = summary_result[opt]['value']
    skipass_mem = summary_result['skipass_value']
    summary_result[opt]['comp'] = "{:.1f}".format(100.0 * (mem-skipass_mem)/mem)

skp_membench_results.append(summary_result)

template_loader = jinja2.FileSystemLoader(searchpath = "/")
template_env = jinja2.Environment( loader=template_loader)

template = template_env.get_template(report_template_filepath)
template_vars = {
    "title": os.path.basename(args.report_dir),
    "skp_membench_opts": opts,
    "skp_membench_results": skp_membench_results,
}

with open(os.path.join(args.report_dir, "index.html"), "w") as f:
    f.write(template.render(template_vars))
