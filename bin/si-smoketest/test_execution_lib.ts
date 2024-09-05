// This library can be used to handle test report output specifically
// to pass it back up to the user, i.e.
// setting thread/parallelisation numbers
// setting test execution strategy, i.e.
//   - linear
//   - soak
//   - ramp
//   - one-shot

import { V4 } from "https://deno.land/x/uuid@v0.1.2/mod.ts";

export interface TestReportEntry {
    test_name: string;
    start_time: string;
    finish_time: string;
    test_duration: string;
    test_result: "success" | "failure";
    message?: string;
    test_execution_sequence: number;
    uuid: string;   
}

let executionCount = 0;

export function createDefaultTestReportEntry(testName: string): TestReportEntry {
    executionCount++;
    return {
      test_name: testName,
      start_time: new Date().toISOString(),
      finish_time: "",
      test_duration: "",
      test_result: "failure",
      message: "",
      test_execution_sequence: executionCount,
      uuid: V4.uuid(),
    };
  }

export function printTestReport(report: TestReportEntry[]) {
    console.log("Test Report:");
    console.log(JSON.stringify(report, null, 2));
}
  
export class ExecutionTracker {
        
    private reports: TestReport[] = [];

    startTest(testName: string): TestReport {
        const startTime = new Date();
        return {
        test_name: testName,
        start_time: startTime.toISOString(),
        finish_time: "",
        test_duration: "",
        test_result: "",
        };
    }

    finishTest(report: TestReport, result: string): void {
        const finishTime = new Date();
        const startTime = new Date(report.start_time);
        report.finish_time = finishTime.toISOString();
        report.test_duration = `${finishTime.getTime() - startTime.getTime()}ms`;
        report.test_result = result;
        this.reports.push(report);
    }

    getReports(): TestReport[] {
        return this.reports;
    }
}