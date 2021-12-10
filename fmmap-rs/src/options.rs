macro_rules! declare_and_impl_options {
    ($name: ident, $file_open_options: ident) => {
        #[derive(Clone)]
        pub struct $name {
            pub(crate) mmap_opts: MmapOptions,
            pub(crate) file_opts: $file_open_options,
            pub(crate) max_size: u64,
            pub(crate) remove_on_drop: bool,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            /// Creates a new set of options for configuring and creating a memory map.
            pub fn new() -> Self {
                Self {
                    mmap_opts: MmapOptions::new(),
                    file_opts: <$file_open_options>::new(),
                    max_size: 0,
                    remove_on_drop: false,
                }
            }

            /// Configures the memory map to start at byte offset from the beginning of the file.
            /// This option has no effect on anonymous memory maps.
            /// By default, the offset is 0.
            pub fn offset(&mut self, offset: u64) -> &mut Self {
                &self.mmap_opts = self.mmap_opts.offset(offset);
                self
            }

            /// Configures the created memory mapped buffer to be len bytes long.
            /// This option is mandatory for anonymous memory maps.
            /// For file-backed memory maps, the length will default to the file length.
            pub fn len(&mut self, len: usize) -> &mut Self {
                &self.mmap_opts = self.mmap_opts.len(len);
                self
            }

            /// Populate (prefault) page tables for a mapping.
            /// For a file mapping, this causes read-ahead on the file. This will help to reduce blocking on page faults later.
            /// This option corresponds to the MAP_POPULATE flag on Linux. It has no effect on Windows
            pub fn populate(&mut self) -> &mut Self {
                &self.mmap_opts = self.mmap_opts.populate();
                self
            }

            /// Configures the anonymous memory map to be suitable for a process or thread stack.
            /// This option corresponds to the MAP_STACK flag on Linux. It has no effect on Windows.
            /// This option has no effect on file-backed memory maps
            pub fn stack(&mut self) -> &mut Self {
                &self.mmap_opts = self.mmap_opts.stack();
                self
            }

            /// Configures the max size of the file.
            /// This option only has effect when mmaping a real file in write mode.
            pub fn max_size(&mut self, max_sz: u64) -> &mut Self {
                self.max_size = max_sz;
                self
            }

            /// Configures whether remove the file when .
            /// This option only has effect when mmaping a real file in write mode.
            pub fn remove_on_drop(&mut self, val: bool) -> &mut Self {
                self.remove_on_drop = val;
                self
            }

            pub fn read(&mut self, val: bool) -> &mut Self {
                &self.file_opts = self.file_opts.read(val);
                self
            }

            pub fn write(&mut self, val: bool) -> &mut Self {
                &self.file_opts = self.file_opts.write(val);
                self
            }

            pub fn create(&mut self, val: bool) -> &mut Self {
                &self.file_opts = self.file_opts.create(val);
                self
            }

            pub fn create_new(&mut self, val: bool) -> &mut Self {
                &self.file_opts = self.file_opts.create_new(val);
                self
            }

            pub fn append(&mut self, val: bool) -> &mut Self {
                &self.file_opts = self.file_opts.append(val);
                self
            }

            pub fn truncate(&mut self, val: bool) -> &mut Self {
                &self.file_opts = self.file_opts.truncate(val);
                self
            }

            /// Sets the mode bits that a new file will be created with. [Read more]
            ///
            /// [Read more]: https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode
            #[cfg(unix)]
            pub fn mode(&mut self, mode: u32) -> &mut Self {
                &self.file_opts = self.file_opts.mode(mode);
                self
            }

            /// Pass custom flags to the `flags` argument of `open`. [Read more]
            ///
            /// [Read more]: https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html#tymethod.mode
            #[cfg(unix)]
            pub fn custom_flags(&mut self, flags: i32) -> &mut Self {
                &self.file_opts = self.file_opts.custom_flags(flags);
                self
            }

            /// Overrides the `dwDesiredAccess` argument to the call to [`CreateFile`] with the specified value. [Read more]
            ///
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn access_mode(&mut self, access: u32) -> &mut Self {
                &self.file_opts = self.file_opts.access_mode(access);
                self
            }

            /// Overrides the `dwShareMode` argument to the call to [`CreateFile`] with the specified value. [Read more]
            ///
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn share_mode(&mut self, val: u32) -> &mut Self {
                &self.file_opts = self.file_opts.share_mode(val);
                self
            }

            /// Sets extra flags for the dwFileFlags argument to the
            /// call to [`CreateFile2`] to the specified value (or combines
            /// it with `attributes` and `security_qos_flags` to set the `dwFlagsAndAttributes` for [`CreateFile`]). [Read more]
            ///
            /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn custom_flags(&mut self, flag: u32) -> &mut Self {
                &self.file_opts = self.file_opts.custom_flags(flag);
                self
            }

            /// Overrides the `dwDesiredAccess` argument to the call to [`CreateFile`] with the specified value. [Read more]
            ///
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn attributes(&mut self, val: u32) -> &mut Self {
                &self.file_opts = self.file_opts.attributes(val);
                self
            }

            /// Sets the `dwSecurityQosFlags` argument to the call to
            /// [`CreateFile2`] to the specified value (or combines it with `custom_flags`
            /// and `attributes` to set the `dwFlagsAndAttributes` for [`CreateFile`]). [Read more]
            ///
            /// [`CreateFile2`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfile2
            /// [`CreateFile`]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
            /// [Read more]: https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags
            #[cfg(windows)]
            pub fn security_qos_flags(&mut self, flags: u32) -> &mut Self {
                &self.file_opts = self.file_opts.security_qos_flags(flags);
                self
            }
        }
    };
}

cfg_tokio!(
    mod tokio_impl;
    pub use tokio_impl::AsyncOptions;
);

cfg_sync!(
    mod sync_impl;
    pub use sync_impl::Options;
);
