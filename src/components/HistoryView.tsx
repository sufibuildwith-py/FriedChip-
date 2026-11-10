import React, { useState } from "react";
import {
  ScrollText,
  Activity,
  Terminal,
  Clock,
  CheckCircle2,
  AlertTriangle,
  ChevronRight,
  ChevronDown,
} from "lucide-react";
import { Operation, OperationLog } from "../types";
import { api } from "../api/tauriBridge";

interface HistoryViewProps {
  operations: Operation[];
}

export const HistoryView: React.FC<HistoryViewProps> = ({ operations }) => {
  const [selectedOpId, setSelectedOpId] = useState<string | null>(null);
  const [logs, setLogs] = useState<Record<string, OperationLog[]>>({});
  const [loadingLogId, setLoadingLogId] = useState<string | null>(null);

  const toggleOpLogs = async (opId: string) => {
    if (selectedOpId === opId) {
      setSelectedOpId(null);
      return;
    }

    setSelectedOpId(opId);
    if (!logs[opId]) {
      setLoadingLogId(opId);
      try {
        const fetchedLogs = await api.getOperationLogs(opId);
        setLogs((prev) => ({ ...prev, [opId]: fetchedLogs }));
      } catch (err) {
        console.error("Failed to fetch logs:", err);
      } finally {
        setLoadingLogId(null);
      }
    }
  };

  const completedCount = operations.filter((op) => op.status === "COMPLETED").length;
  const failedCount = operations.filter((op) => op.status === "FAILED").length;

  return (
    <div className="content-body">
      {/* Journal Overview Card */}
      <div className="card">
        <div className="card-title">
          <span>Operation Audit Journal & Traceability</span>
          <Activity size={15} color="var(--accent-purple)" />
        </div>
        <p style={{ color: "var(--text-muted)", fontSize: "12px", marginBottom: "14px" }}>
          Every firmware ingestion, verification, and transformation event is recorded as an immutable audit record in SQLite.
          Select any operation row to inspect its live execution telemetry and structural logs.
        </p>

        <div style={{ display: "flex", gap: "16px", paddingTop: "8px", borderTop: "1px solid var(--border-subtle)" }}>
          <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
            Total Events: <strong style={{ color: "var(--text-main)" }}>{operations.length}</strong>
          </div>
          <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
            Successful: <strong style={{ color: "var(--accent-emerald)" }}>{completedCount}</strong>
          </div>
          {failedCount > 0 && (
            <div style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>
              Failed: <strong style={{ color: "var(--accent-red)" }}>{failedCount}</strong>
            </div>
          )}
          <div style={{ fontSize: "11.5px", color: "var(--text-dim)", marginLeft: "auto", fontFamily: "var(--font-mono)" }}>
            Storage: SQLite WAL
          </div>
        </div>
      </div>

      {/* Operations Table */}
      <div className="table-container">
        <div className="table-header-bar">
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <ScrollText size={15} color="var(--accent-primary)" />
            <span className="table-header-title">Audited Operations ({operations.length})</span>
          </div>
          <span style={{ fontSize: "11px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>
            Append-only journal &bull; Non-repudiable logs
          </span>
        </div>

        {operations.length === 0 ? (
          <div className="empty-box">
            <ScrollText size={32} className="empty-icon" />
            <div style={{ fontWeight: 600, color: "var(--text-main)" }}>No operations have been recorded yet.</div>
            <div style={{ fontSize: "11.5px", marginTop: "4px" }}>
              Import a firmware file from the Workspace to initiate audit tracking.
            </div>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th style={{ width: "36px" }}></th>
                <th>Operation Type</th>
                <th>Operation ID</th>
                <th>Target SHA-256</th>
                <th>Status</th>
                <th>Execution Time</th>
                <th>Duration</th>
              </tr>
            </thead>
            <tbody>
              {operations.map((op) => {
                const isExpanded = selectedOpId === op.id;
                const opLogs = logs[op.id] || [];
                const startTime = new Date(op.created_at).getTime();
                const endTime = op.completed_at ? new Date(op.completed_at).getTime() : null;
                const durationMs = endTime ? endTime - startTime : null;

                return (
                  <React.Fragment key={op.id}>
                    <tr
                      onClick={() => toggleOpLogs(op.id)}
                      style={{ cursor: "pointer", backgroundColor: isExpanded ? "var(--bg-card-hover)" : undefined }}
                    >
                      <td style={{ color: "var(--text-dim)", textAlign: "center" }}>
                        {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                      </td>
                      <td>
                        <span style={{ fontWeight: 600, color: "var(--text-main)" }}>{op.operation_type}</span>
                      </td>
                      <td className="mono-hash">{op.id.substring(0, 8)}...</td>
                      <td className="mono-hash">
                        {op.input_sha256 ? `${op.input_sha256.substring(0, 16)}...` : "—"}
                      </td>
                      <td>
                        <span className={`badge ${op.status === "COMPLETED" ? "badge-completed" : "badge-failed"}`}>
                          {op.status === "COMPLETED" ? (
                            <CheckCircle2 size={11} style={{ marginRight: "4px" }} />
                          ) : (
                            <AlertTriangle size={11} style={{ marginRight: "4px" }} />
                          )}
                          {op.status}
                        </span>
                      </td>
                      <td style={{ color: "var(--text-muted)", fontSize: "11px" }}>
                        <Clock size={11} style={{ display: "inline", marginRight: "4px", verticalAlign: "middle" }} />
                        {new Date(op.created_at).toLocaleTimeString()}
                      </td>
                      <td style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--text-dim)" }}>
                        {durationMs !== null ? `${durationMs} ms` : "In progress"}
                      </td>
                    </tr>

                    {isExpanded && (
                      <tr>
                        <td colSpan={7} style={{ padding: "14px 20px", backgroundColor: "var(--bg-subtle)" }}>
                          <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "10px", color: "var(--text-muted)", fontSize: "11.5px" }}>
                            <Terminal size={14} color="var(--accent-primary)" />
                            <span>Execution Log Telemetry (Operation ID: <code>{op.id}</code>)</span>
                          </div>

                          {loadingLogId === op.id ? (
                            <div style={{ padding: "14px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>
                              Loading execution logs...
                            </div>
                          ) : opLogs.length === 0 ? (
                            <div style={{ padding: "14px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>
                              No log lines recorded for this operation.
                            </div>
                          ) : (
                            <div className="log-terminal">
                              {opLogs.map((log) => (
                                <div key={log.id} className="log-line">
                                  <span className="log-time">
                                    [{new Date(log.timestamp).toISOString().substring(11, 23)}]
                                  </span>
                                  <span
                                    className={
                                      log.level === "ERROR"
                                        ? "log-error"
                                        : log.level === "WARN"
                                        ? "log-warn"
                                        : "log-info"
                                    }
                                  >
                                    [{log.level}]
                                  </span>
                                  <span className="log-msg">{log.message}</span>
                                </div>
                              ))}
                            </div>
                          )}
                        </td>
                      </tr>
                    )}
                  </React.Fragment>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
};
