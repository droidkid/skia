import sys
import csv
import os
import jinja2
import argparse
import skia_opt_metrics_pb2 as SkiaOptMetrics

'''
The script will write an index.html in the report directory generated as a result 
of a skia_opt_membench run.

This script assumes that the report directory has a benchmark.pb containing a proto of
SkiaOptMetrics and summarizes those results to be rendered in a HTML template.

Usage
python gen_report.py -d <dir_from_membench>
'''

parser = argparse.ArgumentParser(description='Generate a HTML report of skp_opt_membench results.')
parser.add_argument('-d', '--report_dir', help='directory containing results of a skp_opt_membench run')
parser.add_argument('-t', '--report_template', default='report_template.html', help='path to the html template')
parser.add_argument('-b', '--benchmarks', default='benchmark', help='Space seperated list of benchmarks. The report generator will look for <benchmark>.pb files in the <report_dir>')

opts = {
    'NO_OPT': {
        'display_name': 'Baseline'
    },
    'SKIA_RECORD_OPTS': {
        'display_name': 'Existing'
    },
    'SKI_PASS': {
        'display_name': 'SkiPass'
    }
}

def pretty_byte_str(b):
    if b>=(2**10) and b<(2**20):
        return "{:.2f}".format(b/2**10) + "K"
    if b>=(2**20):
        return "{:.2f}".format(b/(2**20)) + "M"
    # One day, we'll work with Gigabyte or greater SKPs, until then...
    return str(b)

args = parser.parse_args()

def build_benchmark_template_vars(benchmark_name):
    proto_summary_filepath = os.path.join(args.report_dir, benchmark_name + '.pb');
    proto_file = open(proto_summary_filepath, "rb")
    proto_data = proto_file.read()
    proto_file.close()

    benchmark = SkiaOptMetrics.SkiaOptBenchmark()
    benchmark.ParseFromString(proto_data)

    skp_results = []

    summary = {}
    summary['name'] = 'SUMMARY'
    summary['ref_img_url'] = ('#')
    summary['skipass_log'] = ('#')
    summary['skipass_value'] = 0
    summary['skipass_link'] = '#'

    for opt in opts:
        summary[opt] = {}
        summary[opt]['bytes'] = 0
        summary[opt]['log'] = '#'

    for skp_benchmark in benchmark.skp_benchmark_runs:
        skp_name = os.path.basename(skp_benchmark.skp_name)
        skp = {}
        skp['name'] = skp_name
        skp['display_name'] = skp_name[:-4] if skp_name.endswith('.skp') else skp_name
        skp['skp_no_opt_url'] = ('%s_NO_OPT.skp' % skp_name)
        skp['skp_ski_pass_url'] = ('%s_SKI_PASS.skp' % skp_name)
        skp['ref_img_url'] = ("NO_OPT_renders/%s.png" % skp_name)
        skp['skipass_log'] = ('./%s_SKI_PASS_SkiPassRunResult.txt' % (skp_name))
        for opt_benchmark in skp_benchmark.optimization_benchmark_runs:
            opt = SkiaOptMetrics.Optimization.Name(opt_benchmark.optimization_type)
            opt_mem = int(opt_benchmark.malloc_allocated_bytes) 
            skp[opt] = {}
            skp[opt]['bytes'] = pretty_byte_str(opt_mem)
            skp[opt]['log'] = ('./%s_%s_log.txt' % (skp['name'], opt)) 
            skp[opt]['img'] = ('./%s_renders/%s.png' % (opt, skp['name'])) 
            skp[opt]['skp'] = ('./%s_renders/%s_%s.skp' % (opt, skp['name'], opt)) 
            summary[opt]['bytes'] += opt_mem
        skp_results.append(skp)

    # Prettify summary byte numbers.
    for opt in opts:
        summary[opt]['bytes'] = pretty_byte_str(summary[opt]['bytes'])

    return {
        "title": benchmark_name,
        "summary": summary,
        "skps": skp_results,
    }



benchmark_list = args.benchmarks.split()
benchmark_template_vars = [build_benchmark_template_vars(b) for b in benchmark_list]

report_template_filepath = os.path.abspath(args.report_template)
template_loader = jinja2.FileSystemLoader(searchpath = "/")
template_env = jinja2.Environment( loader=template_loader)
template = template_env.get_template(report_template_filepath)
template_vars = {
    "benchmarks": benchmark_template_vars,
    "skp_membench_opts": opts,
}

with open(os.path.join(args.report_dir, "index.html"), "w") as f:
    f.write(template.render(template_vars))
