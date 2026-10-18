# Package Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable direct package-manager installation for `sltop` across Linux (Snap Store, APT repository, Fedora Copr/RPM).

**Architecture:** 
1. Canonical Snapcraft configuration (`snapcraft.yaml`) and GitHub Action workflow to build and deploy `.snap` packages for all Linux distros (`snap install sltop`).
2. GitHub Pages hosted APT repository with automated workflow to index and sign `.deb` releases, allowing `sudo apt update && sudo apt install sltop`.
3. RPM spec file and automated setup for Fedora Copr (`dnf copr enable ... && dnf install sltop`).

**Tech Stack:** Snapcraft, APT (reprepro/dpkg-scanpackages), GPG, RPM spec, GitHub Actions.

---

### Task 1: Canonical Snap Store Packaging (`snap install sltop`)
**Files:**
- Create: `snap/snapcraft.yaml`
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Create `snap/snapcraft.yaml`** with classic confinement (needed for full system process monitoring like btop/htop).
- [ ] **Step 2: Add Snap build & publish job** to GitHub Actions.
- [ ] **Step 3: Commit and verify syntax**.

---

### Task 2: Official APT Repository for Debian/Ubuntu (`apt install sltop`)
**Files:**
- Create: `scripts/generate-apt-repo.sh`
- Create: `.github/workflows/apt-repo.yml`
- Modify: `README.md`

- [ ] **Step 1: Create APT repository generation script** that indexes `sltop-amd64.deb` into a Debian `Packages.gz` and `Release` index.
- [ ] **Step 2: Create GitHub Action workflow** to deploy the APT repo automatically to `gh-pages` on every release.
- [ ] **Step 3: Create 1-command installer script** `setup-apt.sh` so users can add the Scoppiato LAB APT repo in 1 second.

---

### Task 3: Fedora / RHEL Copr Specification (`dnf install sltop`)
**Files:**
- Create: `packaging/copr/sltop.spec`

- [ ] **Step 1: Create valid RPM `.spec` file** matching Fedora packaging guidelines for Rust.
- [ ] **Step 2: Commit and document Copr configuration**.
