import * as React from "react"
import { Download, Video, Music, Zap, Lock, Globe, Target, Activity, RefreshCw } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsList, TabsTab, TabsPanel } from "@/components/ui/tabs"
import { Header } from "@/components/header"
import { Footer } from "@/components/footer"

const features = [
  {
    icon: Video,
    title: "高质量视频下载",
    description: "支持最高 8K 超高清视频下载，享受极致画质体验",
  },
  {
    icon: Music,
    title: "音频提取",
    description: "支持下载 MP3 格式音频，轻松提取喜爱的音乐",
  },
  {
    icon: Lock,
    title: "登录支持",
    description: "二维码登录获取高清视频权限，解锁更多画质选项",
  },
  {
    icon: Zap,
    title: "多线程下载",
    description: "基于 aria2c 的高速多线程下载，极速获取视频",
  },
  {
    icon: Globe,
    title: "多语言支持",
    description: "支持中文/英文界面切换，满足不同用户需求",
  },
  {
    icon: Target,
    title: "智能解析",
    description: "支持 BV 号和视频链接解析，多种输入方式",
  },
  {
    icon: Activity,
    title: "下载管理",
    description: "实时进度显示和队列管理，轻松掌控下载任务",
  },
  {
    icon: RefreshCw,
    title: "自动合并",
    description: "视频音频智能合并，一键生成完整视频文件",
  },
]

const videoQualities = [
  { quality: "8K", description: "超高清", needLogin: true },
  { quality: "4K", description: "超清", needLogin: true },
  { quality: "1080P 60帧", description: "高帧率", needLogin: true },
  { quality: "1080P+", description: "高码率", needLogin: true },
  { quality: "1080P", description: "高清", needLogin: true },
  { quality: "720P", description: "高清", needLogin: true },
  { quality: "480P", description: "清晰", needLogin: false },
  { quality: "360P", description: "流畅", needLogin: false },
]

const usageSteps = [
  {
    step: "1",
    title: "下载发行版",
    description: "从 GitHub Releases 页面下载最新版本的压缩包",
  },
  {
    step: "2",
    title: "解压文件",
    description: "将下载的压缩包解压到任意目录",
  },
  {
    step: "3",
    title: "运行程序",
    description: "双击 bilibili-down.exe 启动程序",
  },
  {
    step: "4",
    title: "输入视频信息",
    description: "支持 BV 号、完整链接或短链接",
  },
  {
    step: "5",
    title: "解析视频",
    description: "点击「解析」按钮获取视频信息",
  },
  {
    step: "6",
    title: "选择质量并下载",
    description: "在弹出窗口中选择所需的视频质量，开始下载",
  },
]

export default function Home() {
  return (
    <div className="flex min-h-screen flex-col">
      <Header />

      <main className="flex-1">
        {/* Hero Section */}
        <section className="w-full py-12 md:py-24 lg:py-32">
          <div className="container mx-auto px-4">
            <div className="flex flex-col items-center space-y-6 text-center">
              <Badge variant="secondary" className="text-sm">
                现代化视频下载工具
              </Badge>
              <h1 className="text-4xl font-bold tracking-tighter sm:text-5xl md:text-6xl lg:text-7xl">
                Bilibili-Down
              </h1>
              <p className="max-w-[700px] text-lg text-muted-foreground md:text-xl">
                一个现代化的哔哩哔哩视频下载工具，使用 Rust 开发，基于 egui 图形界面框架构建。
                支持高质量视频下载、音频提取、多线程下载等强大功能。
              </p>
              <div className="flex flex-col gap-4 sm:flex-row">
                <Button
                  size="lg"
                  render={
                    <a
                      href="https://github.com/NORMAL-EX/Bilibili-Down/releases"
                      target="_blank"
                      rel="noopener noreferrer"
                    />
                  }
                >
                  <Download className="mr-2 h-5 w-5" />
                  立即下载
                </Button>
                <Button
                  size="lg"
                  variant="outline"
                  render={
                    <a
                      href="https://github.com/NORMAL-EX/Bilibili-Down"
                      target="_blank"
                      rel="noopener noreferrer"
                    />
                  }
                >
                  查看源码
                </Button>
              </div>
              <div className="flex flex-wrap items-center justify-center gap-4 text-sm text-muted-foreground">
                <Badge variant="outline">Windows 10/11</Badge>
                <Badge variant="outline">MIT License</Badge>
                <Badge variant="outline">Rust</Badge>
              </div>
            </div>
          </div>
        </section>

        {/* Features Section */}
        <section className="w-full bg-muted/50 py-12 md:py-24">
          <div className="container mx-auto px-4">
            <div className="flex flex-col items-center space-y-8">
              <div className="text-center">
                <h2 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
                  强大的功能特性
                </h2>
                <p className="mt-4 text-muted-foreground">
                  为您提供最佳的视频下载体验
                </p>
              </div>
              <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
                {features.map((feature, index) => (
                  <Card key={index}>
                    <CardHeader>
                      <feature.icon className="h-10 w-10 mb-2 text-primary" />
                      <CardTitle className="text-lg">{feature.title}</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <CardDescription>{feature.description}</CardDescription>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* Video Quality Section */}
        <section className="w-full py-12 md:py-24">
          <div className="container mx-auto px-4">
            <div className="flex flex-col items-center space-y-8">
              <div className="text-center">
                <h2 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
                  支持的视频质量
                </h2>
                <p className="mt-4 text-muted-foreground">
                  从流畅到超高清，满足不同需求
                </p>
              </div>
              <div className="w-full max-w-4xl">
                <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-4">
                  {videoQualities.map((quality, index) => (
                    <Card key={index} className="text-center">
                      <CardHeader>
                        <CardTitle className="text-2xl">{quality.quality}</CardTitle>
                        <CardDescription>{quality.description}</CardDescription>
                      </CardHeader>
                      <CardContent>
                        {quality.needLogin ? (
                          <Badge variant="secondary">需要登录</Badge>
                        ) : (
                          <Badge variant="outline">无需登录</Badge>
                        )}
                      </CardContent>
                    </Card>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Usage Guide Section */}
        <section className="w-full bg-muted/50 py-12 md:py-24">
          <div className="container mx-auto px-4">
            <div className="flex flex-col items-center space-y-8">
              <div className="text-center">
                <h2 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
                  使用指南
                </h2>
                <p className="mt-4 text-muted-foreground">
                  简单几步，轻松上手
                </p>
              </div>
              <Tabs defaultValue="install" className="w-full max-w-4xl">
                <TabsList className="grid w-full grid-cols-2">
                  <TabsTab value="install">安装步骤</TabsTab>
                  <TabsTab value="usage">基础使用</TabsTab>
                </TabsList>
                <TabsPanel value="install" className="mt-6">
                  <div className="grid gap-6 md:grid-cols-3">
                    {usageSteps.slice(0, 3).map((item, index) => (
                      <Card key={index}>
                        <CardHeader>
                          <div className="flex items-center space-x-2">
                            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-primary-foreground font-bold">
                              {item.step}
                            </div>
                            <CardTitle className="text-lg">{item.title}</CardTitle>
                          </div>
                        </CardHeader>
                        <CardContent>
                          <CardDescription>{item.description}</CardDescription>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </TabsPanel>
                <TabsPanel value="usage" className="mt-6">
                  <div className="grid gap-6 md:grid-cols-3">
                    {usageSteps.slice(3).map((item, index) => (
                      <Card key={index}>
                        <CardHeader>
                          <div className="flex items-center space-x-2">
                            <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary text-primary-foreground font-bold">
                              {item.step}
                            </div>
                            <CardTitle className="text-lg">{item.title}</CardTitle>
                          </div>
                        </CardHeader>
                        <CardContent>
                          <CardDescription>{item.description}</CardDescription>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                </TabsPanel>
              </Tabs>
            </div>
          </div>
        </section>

        {/* Technology Stack Section */}
        <section className="w-full py-12 md:py-24">
          <div className="container mx-auto px-4">
            <div className="flex flex-col items-center space-y-8">
              <div className="text-center">
                <h2 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
                  技术栈
                </h2>
                <p className="mt-4 text-muted-foreground">
                  基于现代化技术构建
                </p>
              </div>
              <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-6 w-full max-w-4xl">
                {[
                  { name: "egui", description: "即时模式GUI" },
                  { name: "reqwest", description: "异步HTTP库" },
                  { name: "tokio", description: "异步运行时" },
                  { name: "serde", description: "JSON序列化" },
                  { name: "aria2", description: "多线程下载" },
                  { name: "FFmpeg", description: "音视频处理" },
                ].map((tech, index) => (
                  <Card key={index} className="text-center">
                    <CardHeader>
                      <CardTitle className="text-base">{tech.name}</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <CardDescription className="text-xs">{tech.description}</CardDescription>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* CTA Section */}
        <section className="w-full bg-muted/50 py-12 md:py-24">
          <div className="container mx-auto px-4">
            <div className="flex flex-col items-center space-y-6 text-center">
              <h2 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
                准备开始了吗？
              </h2>
              <p className="max-w-[600px] text-muted-foreground md:text-lg">
                立即下载 Bilibili-Down，体验高效便捷的视频下载服务
              </p>
              <Button
                size="lg"
                render={
                  <a
                    href="https://github.com/NORMAL-EX/Bilibili-Down/releases"
                    target="_blank"
                    rel="noopener noreferrer"
                  />
                }
              >
                <Download className="mr-2 h-5 w-5" />
                前往 Releases 下载
              </Button>
            </div>
          </div>
        </section>
      </main>

      <Footer />
    </div>
  )
}
