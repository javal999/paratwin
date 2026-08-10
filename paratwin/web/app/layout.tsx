import type { Metadata } from "next";
import "./globals.css";
import Nav from "@/components/Nav";

export const metadata: Metadata = {
  title: "ParaTwin — ParaExpress Digital Twin",
  description:
    "ParagonCorp digital twin simulator for the ParaExpress hyperlocal fulfillment network",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <div className="flex min-h-screen">
          <Nav />
          <main className="flex-1 px-8 py-6 max-w-[1400px]">{children}</main>
        </div>
      </body>
    </html>
  );
}
