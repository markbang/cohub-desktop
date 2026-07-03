import { useState } from "react";
import {
  Banner,
  Button,
  Empty,
  Input,
  LayerCard,
  Text,
} from "@cloudflare/kumo";
import { FolderSimple, HardDrives } from "@phosphor-icons/react";
import { cohub } from "../lib/api";
import { toastManager } from "../lib/toast";
import type { SpaceInfo } from "../lib/types";

interface Props {
  loggedIn: boolean;
  spaces: SpaceInfo[];
}

function defaultPath(name: string) {
  return `~/cohub/${name}`;
}

export function CloudDrive({ loggedIn, spaces }: Props) {
  const [paths, setPaths] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);

  const pathOf = (s: SpaceInfo) => paths[s.id] ?? defaultPath(s.name);

  const mount = async (s: SpaceInfo) => {
    const p = pathOf(s);
    setBusy(s.id);
    try {
      await cohub.mountSpace(s.id, p);
    } catch (e) {
      toastManager.add({ title: "挂载失败", description: String(e), variant: "error" });
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="shrink-0 px-6 py-5">
        <Text variant="heading2" as="h1">
          云盘挂载
        </Text>
        <Text variant="secondary" size="sm">
          把 Space 当作云盘挂载到本地，像操作本地文件一样管理对话产物。
        </Text>
      </header>

      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-6 pb-6">
        <Banner
          variant="default"
          title="Roadmap 能力"
          description="将通过 FUSE（macOS / Linux）与 WinFSP（Windows）把 Space 挂载为本地磁盘。命令链路已接通，后端挂载尚未实现——点击挂载会返回未实现提示。"
        />

        {!loggedIn ? (
          <Empty
            icon={<HardDrives size={40} />}
            title="登录后即可挂载 Space"
            description="先在「实时活动」里登录 Cohub，再回到这里选择要挂载的 Space。"
          />
        ) : spaces.length === 0 ? (
          <Empty title="没有可挂载的 Space" description="你的账号下暂无 Space。" />
        ) : (
          <div className="space-y-3">
            {spaces.map((s) => (
              <LayerCard key={s.id} className="rounded-xl p-4">
                <div className="flex flex-col gap-3 md:flex-row md:items-end">
                  <div className="min-w-0 flex-1">
                    <Text variant="body" size="sm" bold as="p">
                      {s.name}
                    </Text>
                    <span className="block truncate font-mono text-xs text-kumo-subtle">
                      {s.id}
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <div className="w-72">
                      <Input
                        label="挂载路径"
                        value={pathOf(s)}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                          setPaths((p) => ({ ...p, [s.id]: e.target.value }))
                        }
                      />
                    </div>
                    <Button
                      variant="secondary"
                      icon={<FolderSimple size={16} />}
                      loading={busy === s.id}
                      onClick={() => mount(s)}
                    >
                      挂载
                    </Button>
                  </div>
                </div>
              </LayerCard>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
