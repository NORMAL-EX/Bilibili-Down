import type { Metadata } from "next";
import "./globals.css";
import { ThemeProvider } from "@/components/theme-provider";

export const metadata: Metadata = {
  title: "Bilibili-Down - 现代化的哔哩哔哩视频下载工具",
  description: "一个现代化的哔哩哔哩视频下载工具，支持高质量视频下载、音频提取、多线程下载等功能。使用 Rust 开发，基于 egui 图形界面框架构建。",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <body className="antialiased">
        <ThemeProvider defaultTheme="system">
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
