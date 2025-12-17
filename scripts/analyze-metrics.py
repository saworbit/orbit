#!/usr/bin/env python3
"""
Orbit E2E Metrics Analyzer
Analyzes performance metrics from demo runs
Version: 2.2.0-alpha
"""

import json
import sys
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any

class MetricsAnalyzer:
    def __init__(self, metrics_file: str):
        self.metrics_file = Path(metrics_file)
        self.metrics = self._load_metrics()

    def _load_metrics(self) -> Dict[str, Any]:
        """Load metrics from JSON file"""
        if not self.metrics_file.exists():
            raise FileNotFoundError(f"Metrics file not found: {self.metrics_file}")

        with open(self.metrics_file, 'r') as f:
            return json.load(f)

    def analyze(self) -> Dict[str, Any]:
        """Analyze metrics and generate report"""
        analysis = {
            "summary": self._generate_summary(),
            "performance": self._analyze_performance(),
            "health": self._check_health(),
            "recommendations": self._generate_recommendations()
        }
        return analysis

    def _generate_summary(self) -> Dict[str, Any]:
        """Generate high-level summary"""
        return {
            "timestamp": self.metrics.get("timestamp", "unknown"),
            "job_id": self.metrics.get("job_id", "unknown"),
            "total_duration_seconds": self.metrics.get("total_duration_seconds", 0),
            "transfer_success": self.metrics.get("transfer_success", False),
            "files_transferred": self.metrics.get("destination_files_count", 0),
            "test_data_size_mb": round(self.metrics.get("test_data_bytes", 0) / 1024 / 1024, 2)
        }

    def _analyze_performance(self) -> Dict[str, Any]:
        """Analyze performance metrics"""
        total_duration = self.metrics.get("total_duration_seconds", 0)
        data_bytes = self.metrics.get("test_data_bytes", 0)

        # Calculate throughput
        throughput_mbps = 0
        if total_duration > 0:
            throughput_mbps = round((data_bytes / 1024 / 1024) / total_duration, 2)

        # Phase breakdown
        phases = {
            "preflight": self.metrics.get("preflight_duration_seconds", 0),
            "data_fabrication": self.metrics.get("data_fabrication_duration_seconds", 0),
            "ignition": self.metrics.get("ignition_duration_seconds", 0),
            "job_creation": self.metrics.get("job_creation_duration_seconds", 0),
            "job_monitoring": self.metrics.get("job_monitoring_duration_seconds", 0),
            "health_check": self.metrics.get("health_check_duration_seconds", 0)
        }

        # Calculate overhead
        overhead = sum(phases.values()) - phases["job_monitoring"]
        actual_transfer_time = phases["job_monitoring"]

        return {
            "throughput_mbps": throughput_mbps,
            "phases": phases,
            "overhead_seconds": overhead,
            "actual_transfer_seconds": actual_transfer_time,
            "overhead_percentage": round((overhead / total_duration * 100), 1) if total_duration > 0 else 0
        }

    def _check_health(self) -> Dict[str, Any]:
        """Check health indicators"""
        issues = []
        warnings = []

        # Check transfer success
        if not self.metrics.get("transfer_success", False):
            issues.append("Transfer failed")

        # Check file count mismatch
        expected = self.metrics.get("test_files_count", 0)
        actual = self.metrics.get("destination_files_count", 0)
        if expected != actual:
            issues.append(f"File count mismatch: expected {expected}, got {actual}")

        # Check for timeouts
        if self.metrics.get("job_status") == "timeout":
            issues.append("Job timed out")

        # Check for long health check
        health_check = self.metrics.get("health_check_duration_seconds", 0)
        if health_check > 30:
            warnings.append(f"Long health check duration: {health_check}s")

        # Check for slow ignition
        ignition = self.metrics.get("ignition_duration_seconds", 0)
        if ignition > 60:
            warnings.append(f"Slow system ignition: {ignition}s")

        return {
            "healthy": len(issues) == 0,
            "issues": issues,
            "warnings": warnings
        }

    def _generate_recommendations(self) -> List[str]:
        """Generate optimization recommendations"""
        recommendations = []

        perf = self._analyze_performance()
        health = self._check_health()

        # Performance recommendations
        if perf["throughput_mbps"] < 10:
            recommendations.append("Low throughput detected. Consider increasing parallel workers.")

        if perf["overhead_percentage"] > 50:
            recommendations.append("High overhead. Optimize startup time or increase test data size.")

        # Health recommendations
        if health["warnings"]:
            for warning in health["warnings"]:
                if "health check" in warning:
                    recommendations.append("Slow API startup. Check for resource constraints.")
                if "ignition" in warning:
                    recommendations.append("Slow system ignition. Pre-build binaries to reduce startup time.")

        if not recommendations:
            recommendations.append("Performance looks good! No recommendations at this time.")

        return recommendations

    def print_report(self):
        """Print formatted analysis report"""
        analysis = self.analyze()

        print("=" * 80)
        print("ORBIT E2E METRICS ANALYSIS REPORT")
        print("=" * 80)
        print()

        # Summary
        print("📊 SUMMARY")
        print("-" * 80)
        summary = analysis["summary"]
        print(f"  Timestamp:       {summary['timestamp']}")
        print(f"  Job ID:          {summary['job_id']}")
        print(f"  Total Duration:  {summary['total_duration_seconds']}s")
        print(f"  Success:         {'✓' if summary['transfer_success'] else '✗'}")
        print(f"  Files:           {summary['files_transferred']}")
        print(f"  Data Size:       {summary['test_data_size_mb']} MB")
        print()

        # Performance
        print("⚡ PERFORMANCE")
        print("-" * 80)
        perf = analysis["performance"]
        print(f"  Throughput:      {perf['throughput_mbps']} MB/s")
        print(f"  Overhead:        {perf['overhead_seconds']}s ({perf['overhead_percentage']}%)")
        print(f"  Transfer Time:   {perf['actual_transfer_seconds']}s")
        print()
        print("  Phase Breakdown:")
        for phase, duration in perf["phases"].items():
            print(f"    {phase:20s} {duration:6.2f}s")
        print()

        # Health
        print("🏥 HEALTH")
        print("-" * 80)
        health = analysis["health"]
        print(f"  Status:          {'✓ HEALTHY' if health['healthy'] else '✗ ISSUES FOUND'}")
        if health["issues"]:
            print("  Issues:")
            for issue in health["issues"]:
                print(f"    ✗ {issue}")
        if health["warnings"]:
            print("  Warnings:")
            for warning in health["warnings"]:
                print(f"    ⚠ {warning}")
        if not health["issues"] and not health["warnings"]:
            print("  No issues or warnings detected.")
        print()

        # Recommendations
        print("💡 RECOMMENDATIONS")
        print("-" * 80)
        for rec in analysis["recommendations"]:
            print(f"  • {rec}")
        print()
        print("=" * 80)

    def export_json(self, output_file: str):
        """Export analysis to JSON"""
        analysis = self.analyze()
        with open(output_file, 'w') as f:
            json.dump(analysis, f, indent=2)
        print(f"Analysis exported to {output_file}")

    def export_markdown(self, output_file: str):
        """Export analysis to Markdown"""
        analysis = self.analyze()

        md = []
        md.append("# Orbit E2E Metrics Analysis Report")
        md.append("")
        md.append(f"**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        md.append("")

        # Summary
        md.append("## 📊 Summary")
        md.append("")
        summary = analysis["summary"]
        md.append(f"- **Timestamp:** {summary['timestamp']}")
        md.append(f"- **Job ID:** {summary['job_id']}")
        md.append(f"- **Total Duration:** {summary['total_duration_seconds']}s")
        md.append(f"- **Success:** {'✅' if summary['transfer_success'] else '❌'}")
        md.append(f"- **Files Transferred:** {summary['files_transferred']}")
        md.append(f"- **Data Size:** {summary['test_data_size_mb']} MB")
        md.append("")

        # Performance
        md.append("## ⚡ Performance")
        md.append("")
        perf = analysis["performance"]
        md.append(f"- **Throughput:** {perf['throughput_mbps']} MB/s")
        md.append(f"- **Overhead:** {perf['overhead_seconds']}s ({perf['overhead_percentage']}%)")
        md.append(f"- **Transfer Time:** {perf['actual_transfer_seconds']}s")
        md.append("")
        md.append("### Phase Breakdown")
        md.append("")
        md.append("| Phase | Duration (s) |")
        md.append("|-------|--------------|")
        for phase, duration in perf["phases"].items():
            md.append(f"| {phase} | {duration:.2f} |")
        md.append("")

        # Health
        md.append("## 🏥 Health")
        md.append("")
        health = analysis["health"]
        md.append(f"**Status:** {'✅ HEALTHY' if health['healthy'] else '❌ ISSUES FOUND'}")
        md.append("")
        if health["issues"]:
            md.append("### Issues")
            for issue in health["issues"]:
                md.append(f"- ❌ {issue}")
            md.append("")
        if health["warnings"]:
            md.append("### Warnings")
            for warning in health["warnings"]:
                md.append(f"- ⚠️ {warning}")
            md.append("")

        # Recommendations
        md.append("## 💡 Recommendations")
        md.append("")
        for rec in analysis["recommendations"]:
            md.append(f"- {rec}")
        md.append("")

        with open(output_file, 'w') as f:
            f.write('\n'.join(md))
        print(f"Analysis exported to {output_file}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze-metrics.py <metrics.json> [--export-json output.json] [--export-md output.md]")
        print()
        print("Examples:")
        print("  python analyze-metrics.py e2e-metrics.json")
        print("  python analyze-metrics.py e2e-metrics.json --export-json analysis.json")
        print("  python analyze-metrics.py e2e-metrics.json --export-md report.md")
        sys.exit(1)

    metrics_file = sys.argv[1]

    try:
        analyzer = MetricsAnalyzer(metrics_file)
        analyzer.print_report()

        # Check for export options
        if "--export-json" in sys.argv:
            idx = sys.argv.index("--export-json")
            if idx + 1 < len(sys.argv):
                analyzer.export_json(sys.argv[idx + 1])

        if "--export-md" in sys.argv:
            idx = sys.argv.index("--export-md")
            if idx + 1 < len(sys.argv):
                analyzer.export_markdown(sys.argv[idx + 1])

    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in metrics file: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Unexpected error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
