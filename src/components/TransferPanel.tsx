import { useEffect, useState } from "react";
import {
  Badge,
  Button,
  Progress,
  Tooltip,
  Typography,
  Space,
  theme,
} from "antd";
import {
  UploadOutlined,
  DownloadOutlined,
  CloseOutlined,
  CloseCircleOutlined,
  ReloadOutlined,
  SwapOutlined,
} from "@ant-design/icons";
import type { UploadTask, DownloadTask } from "../types";

const { Text } = Typography;

interface Props {
  uploads: UploadTask[];
  downloads: DownloadTask[];
  onDismissUpload: (id: string) => void;
  onDismissDownload: (id: string) => void;
  onClearAll: () => void;
}

export function TransferPanel({
  uploads,
  downloads,
  onDismissUpload,
  onDismissDownload,
  onClearAll,
}: Props) {
  const { token } = theme.useToken();
  const [expanded, setExpanded] = useState(false);

  // Auto-expand when a new active transfer appears.
  const activeCount = uploads.filter((u) => !u.done).length + downloads.filter((d) => !d.done).length;
  useEffect(() => {
    if (activeCount > 0) setExpanded(true);
  }, [activeCount]);

  const totalCount = uploads.length + downloads.length;

  if (totalCount === 0) return null;

  return (
    <div
      style={{
        position: "fixed",
        bottom: 24,
        right: 24,
        zIndex: 1000,
        display: "flex",
        flexDirection: "column",
        alignItems: "flex-end",
        gap: 8,
      }}
    >
      {/* Expanded panel */}
      {expanded && (
        <div
          className="transfer-panel-card"
          style={{
            width: 340,
            maxHeight: 480,
            borderRadius: token.borderRadiusLG,
            boxShadow: token.boxShadowSecondary,
            background: token.colorBgContainer,
            border: `1px solid ${token.colorBorderSecondary}`,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          {/* Header */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              padding: "10px 14px",
              borderBottom: `1px solid ${token.colorBorderSecondary}`,
              background: token.colorFillAlter,
              flexShrink: 0,
            }}
          >
            <Text strong style={{ flex: 1, fontSize: 13 }}>
              Transfers
              {activeCount > 0 && (
                <Text type="secondary" style={{ fontSize: 12, marginLeft: 6 }}>
                  {activeCount} active
                </Text>
              )}
            </Text>
            <Space size={4}>
              {totalCount > 0 && (
                <Tooltip title="Clear completed">
                  <Button
                    size="small"
                    type="text"
                    onClick={onClearAll}
                    style={{ fontSize: 12, color: token.colorTextSecondary }}
                  >
                    Clear
                  </Button>
                </Tooltip>
              )}
              <Button
                size="small"
                type="text"
                icon={<CloseOutlined />}
                onClick={() => setExpanded(false)}
              />
            </Space>
          </div>

          {/* Task list */}
          <div style={{ overflowY: "auto", padding: "8px 14px", flex: 1 }}>
            {uploads.map((u) => (
              <TaskRow
                key={u.id}
                icon={<UploadOutlined style={{ color: token.colorPrimary, fontSize: 12 }} />}
                filename={u.filename}
                progress={u.progress}
                done={u.done}
                error={u.error}
                actions={
                  u.error ? (
                    <Space size={2}>
                      {u.retry && (
                        <Tooltip title="Retry">
                          <Button size="small" type="text" icon={<ReloadOutlined />} onClick={u.retry} />
                        </Tooltip>
                      )}
                      <Tooltip title="Dismiss">
                        <Button
                          size="small"
                          type="text"
                          icon={<CloseCircleOutlined />}
                          onClick={() => onDismissUpload(u.id)}
                        />
                      </Tooltip>
                    </Space>
                  ) : null
                }
              />
            ))}

            {downloads.map((d) => (
              <TaskRow
                key={d.id}
                icon={<DownloadOutlined style={{ color: token.colorSuccess, fontSize: 12 }} />}
                filename={d.filename}
                progress={d.progress}
                done={d.done}
                error={d.error}
                actions={
                  d.done ? (
                    <Tooltip title="Dismiss">
                      <Button
                        size="small"
                        type="text"
                        icon={<CloseCircleOutlined />}
                        onClick={() => onDismissDownload(d.id)}
                      />
                    </Tooltip>
                  ) : null
                }
              />
            ))}
          </div>
        </div>
      )}

      {/* FAB trigger */}
      <Tooltip title={expanded ? "Hide transfers" : "Show transfers"} placement="left">
        <Badge count={activeCount} size="small" offset={[-4, 4]}>
          <Button
            type="primary"
            shape="circle"
            size="large"
            className="transfer-fab-btn"
            icon={<SwapOutlined rotate={90} />}
            onClick={() => setExpanded((v) => !v)}
            style={{
              width: 44,
              height: 44,
              boxShadow: token.boxShadow,
              opacity: activeCount > 0 ? 1 : 0.72,
            }}
          />
        </Badge>
      </Tooltip>
    </div>
  );
}

// ─── TaskRow ──────────────────────────────────────────────────────────────────

interface TaskRowProps {
  icon: React.ReactNode;
  filename: string;
  progress: number;
  done: boolean;
  error?: string;
  actions?: React.ReactNode;
}

function TaskRow({ icon, filename, progress, done, error, actions }: TaskRowProps) {
  const { token } = theme.useToken();
  return (
    <div className="transfer-task-row">
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
        {icon}
        <Text
          style={{
            fontSize: 12,
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            color: token.colorText,
          }}
          title={filename}
        >
          {filename}
        </Text>
        {actions}
      </div>
      <Progress
        percent={progress}
        size="small"
        status={error ? "exception" : done ? "success" : "active"}
        format={() =>
          error ? (
            <Text type="danger" style={{ fontSize: 10 }}>
              {error.length > 30 ? error.slice(0, 30) + "…" : error}
            </Text>
          ) : (
            `${progress}%`
          )
        }
      />
    </div>
  );
}
