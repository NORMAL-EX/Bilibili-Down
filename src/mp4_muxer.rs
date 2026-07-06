use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Copy)]
struct BoxHeader {
    typ: [u8; 4],
    start: usize,
    header_size: usize,
    size: usize,
}

impl BoxHeader {
    fn content_start(&self) -> usize {
        self.start + self.header_size
    }

    fn end(&self) -> usize {
        self.start + self.size
    }
}

#[derive(Clone)]
struct Sample {
    offset: usize,
    size: u32,
    duration: u32,
    cts_offset: i32,
    sync: bool,
}

struct Track {
    source: Vec<u8>,
    samples: Vec<Sample>,
    track_id: u32,
    handler: [u8; 4],
    timescale: u32,
    duration: u64,
    width: u32,
    height: u32,
    stsd_box: Vec<u8>,
}

#[derive(Clone, Copy)]
struct TrexInfo {
    default_duration: Option<u32>,
    default_size: Option<u32>,
    default_flags: Option<u32>,
}

pub fn mux_m4s_to_mp4(video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<(), String> {
    let video = parse_track(video_path, Some(*b"vide"), 1)?;
    let audio = parse_track(audio_path, Some(*b"soun"), 2)?;
    write_mp4(&[video, audio], output_path)
}

fn parse_track(path: &Path, expected_handler: Option<[u8; 4]>, output_track_id: u32) -> Result<Track, String> {
    let source = fs::read(path).map_err(|e| format!("读取 {:?} 失败: {}", path, e))?;
    let moov = find_box(&source, 0, source.len(), b"moov").ok_or_else(|| format!("{:?} 缺少 moov", path))?;

    let mut selected = None;
    let mut offset = moov.content_start();
    while let Some(trak) = read_box(&source, offset, moov.end()) {
        offset = trak.end();
        if &trak.typ != b"trak" {
            continue;
        }

        let hdlr = find_path(&source, trak.content_start(), trak.end(), &[b"mdia", b"hdlr"])
            .ok_or_else(|| format!("{:?} 缺少 hdlr", path))?;
        let handler = read_handler_type(&source, hdlr)?;
        if expected_handler.map_or(true, |expected| expected == handler) {
            selected = Some((trak, handler));
            break;
        }
    }

    let (trak, handler) = selected.ok_or_else(|| format!("{:?} 没有找到匹配的媒体轨", path))?;
    let tkhd = find_box(&source, trak.content_start(), trak.end(), b"tkhd").ok_or_else(|| format!("{:?} 缺少 tkhd", path))?;
    let (source_track_id, width, height) = read_tkhd(&source, tkhd)?;
    let trex = read_trex(&source, moov, source_track_id);

    let mdhd = find_path(&source, trak.content_start(), trak.end(), &[b"mdia", b"mdhd"])
        .ok_or_else(|| format!("{:?} 缺少 mdhd", path))?;
    let timescale = read_mdhd_timescale(&source, mdhd)?;

    let stsd = find_path(&source, trak.content_start(), trak.end(), &[b"mdia", b"minf", b"stbl", b"stsd"])
        .ok_or_else(|| format!("{:?} 缺少 stsd", path))?;
    let stsd_box = source[stsd.start..stsd.end()].to_vec();

    let mut samples = Vec::new();
    let mut cursor = 0;
    while let Some(moof) = find_box(&source, cursor, source.len(), b"moof") {
        parse_moof_samples(&source, moof, source_track_id, handler, trex, &mut samples)?;
        cursor = moof.end();
    }

    if samples.is_empty() {
        return Err(format!("{:?} 没有解析到媒体 sample", path));
    }

    let duration = samples.iter().map(|sample| sample.duration as u64).sum();
    Ok(Track {
        source,
        samples,
        track_id: output_track_id,
        handler,
        timescale,
        duration,
        width,
        height,
        stsd_box,
    })
}

fn parse_moof_samples(
    source: &[u8],
    moof: BoxHeader,
    source_track_id: u32,
    handler: [u8; 4],
    trex: Option<TrexInfo>,
    samples: &mut Vec<Sample>,
) -> Result<(), String> {
    let mut offset = moof.content_start();
    while let Some(traf) = read_box(source, offset, moof.end()) {
        offset = traf.end();
        if &traf.typ != b"traf" {
            continue;
        }

        let tfhd = find_box(source, traf.content_start(), traf.end(), b"tfhd").ok_or("traf 缺少 tfhd")?;
        let mut tfhd_info = read_tfhd(source, tfhd)?;
        if tfhd_info.track_id != source_track_id {
            continue;
        }
        if let Some(trex) = trex {
            tfhd_info.default_duration = tfhd_info.default_duration.or(trex.default_duration);
            tfhd_info.default_size = tfhd_info.default_size.or(trex.default_size);
            tfhd_info.default_flags = tfhd_info.default_flags.or(trex.default_flags);
        }

        let mut trun_offset = traf.content_start();
        while let Some(trun) = find_box(source, trun_offset, traf.end(), b"trun") {
            trun_offset = trun.end();
            read_trun(source, moof, trun, handler, &tfhd_info, samples)?;
        }
    }

    Ok(())
}

struct TfhdInfo {
    track_id: u32,
    default_duration: Option<u32>,
    default_size: Option<u32>,
    default_flags: Option<u32>,
}

fn read_tfhd(source: &[u8], tfhd: BoxHeader) -> Result<TfhdInfo, String> {
    let data = &source[tfhd.content_start()..tfhd.end()];
    if data.len() < 8 {
        return Err("tfhd 太短".to_string());
    }

    let flags = read_u24(&data[1..4]);
    let track_id = read_u32(&data[4..8]);
    let mut pos = 8;

    if flags & 0x000001 != 0 {
        pos += 8;
    }
    if flags & 0x000002 != 0 {
        pos += 4;
    }

    let default_duration = if flags & 0x000008 != 0 {
        let value = read_u32_checked(data, pos, "tfhd default duration")?;
        pos += 4;
        Some(value)
    } else {
        None
    };

    let default_size = if flags & 0x000010 != 0 {
        let value = read_u32_checked(data, pos, "tfhd default size")?;
        pos += 4;
        Some(value)
    } else {
        None
    };

    let default_flags = if flags & 0x000020 != 0 {
        Some(read_u32_checked(data, pos, "tfhd default flags")?)
    } else {
        None
    };

    Ok(TfhdInfo {
        track_id,
        default_duration,
        default_size,
        default_flags,
    })
}

fn read_trun(
    source: &[u8],
    moof: BoxHeader,
    trun: BoxHeader,
    handler: [u8; 4],
    tfhd: &TfhdInfo,
    samples: &mut Vec<Sample>,
) -> Result<(), String> {
    let data = &source[trun.content_start()..trun.end()];
    if data.len() < 8 {
        return Err("trun 太短".to_string());
    }

    let version = data[0];
    let flags = read_u24(&data[1..4]);
    let sample_count = read_u32(&data[4..8]) as usize;
    let mut pos = 8;

    let mut data_offset = find_following_mdat(source, moof).ok_or("moof 后缺少 mdat")? as i64;
    if flags & 0x000001 != 0 {
        data_offset = moof.start as i64 + read_i32_checked(data, pos, "trun data offset")? as i64;
        pos += 4;
    }

    let first_sample_flags = if flags & 0x000004 != 0 {
        let value = read_u32_checked(data, pos, "trun first sample flags")?;
        pos += 4;
        Some(value)
    } else {
        None
    };

    let has_duration = flags & 0x000100 != 0;
    let has_size = flags & 0x000200 != 0;
    let has_flags = flags & 0x000400 != 0;
    let has_cts = flags & 0x000800 != 0;

    if data_offset < 0 {
        return Err("trun data offset 为负数".to_string());
    }

    let mut media_offset = data_offset as usize;
    for index in 0..sample_count {
        let duration = if has_duration {
            let value = read_u32_checked(data, pos, "trun sample duration")?;
            pos += 4;
            value
        } else {
            tfhd.default_duration.ok_or("缺少 sample duration")?
        };

        let size = if has_size {
            let value = read_u32_checked(data, pos, "trun sample size")?;
            pos += 4;
            value
        } else {
            tfhd.default_size.ok_or("缺少 sample size")?
        };

        let flags_value = if has_flags {
            let value = read_u32_checked(data, pos, "trun sample flags")?;
            pos += 4;
            value
        } else if index == 0 {
            first_sample_flags.or(tfhd.default_flags).unwrap_or(0)
        } else {
            tfhd.default_flags.unwrap_or(0)
        };

        let cts_offset = if has_cts {
            let value = if version == 0 {
                read_u32_checked(data, pos, "trun composition offset")? as i32
            } else {
                read_i32_checked(data, pos, "trun composition offset")?
            };
            pos += 4;
            value
        } else {
            0
        };

        let end = media_offset.checked_add(size as usize).ok_or("sample size 溢出")?;
        if end > source.len() {
            return Err("sample 数据越界".to_string());
        }

        let sync = handler != *b"vide" || flags_value & 0x0001_0000 == 0;
        samples.push(Sample {
            offset: media_offset,
            size,
            duration,
            cts_offset,
            sync,
        });
        media_offset = end;
    }

    Ok(())
}

fn write_mp4(tracks: &[Track], output_path: &Path) -> Result<(), String> {
    let ftyp = build_ftyp();
    let mdat_payload_size = tracks
        .iter()
        .flat_map(|track| track.samples.iter())
        .try_fold(0u64, |sum, sample| sum.checked_add(sample.size as u64))
        .ok_or("mdat 体积溢出")?;
    if mdat_payload_size + 8 > u32::MAX as u64 {
        return Err("暂不支持生成超过 4GB 的 MP4".to_string());
    }

    let mut offsets = vec![0u64; tracks.len()];
    let mut moov = build_moov(tracks, &offsets)?;
    let mut first_sample_offset = ftyp.len() as u64 + moov.len() as u64 + 8;
    for (index, track) in tracks.iter().enumerate() {
        offsets[index] = first_sample_offset;
        let track_size: u64 = track.samples.iter().map(|sample| sample.size as u64).sum();
        first_sample_offset += track_size;
    }
    moov = build_moov(tracks, &offsets)?;

    let mut file = fs::File::create(output_path).map_err(|e| format!("创建 {:?} 失败: {}", output_path, e))?;
    file.write_all(&ftyp).map_err(|e| format!("写入 ftyp 失败: {}", e))?;
    file.write_all(&moov).map_err(|e| format!("写入 moov 失败: {}", e))?;
    write_box_header(&mut file, b"mdat", (mdat_payload_size + 8) as u32)?;

    for track in tracks {
        for sample in &track.samples {
            let end = sample.offset + sample.size as usize;
            file.write_all(&track.source[sample.offset..end])
                .map_err(|e| format!("写入 mdat 失败: {}", e))?;
        }
    }

    Ok(())
}

fn build_ftyp() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(b"isom");
    payload.extend(512u32.to_be_bytes());
    payload.extend(b"isom");
    payload.extend(b"iso6");
    payload.extend(b"mp41");
    payload.extend(b"avc1");
    make_box(b"ftyp", &payload)
}

fn build_moov(tracks: &[Track], chunk_offsets: &[u64]) -> Result<Vec<u8>, String> {
    let movie_timescale = 1000u32;
    let movie_duration = tracks
        .iter()
        .map(|track| scale_duration(track.duration, track.timescale, movie_timescale))
        .max()
        .unwrap_or(0);

    let mut payload = Vec::new();
    payload.extend(build_mvhd(movie_timescale, movie_duration, tracks.len() as u32 + 1));
    for (index, track) in tracks.iter().enumerate() {
        payload.extend(build_trak(track, movie_timescale, chunk_offsets[index])?);
    }

    Ok(make_box(b"moov", &payload))
}

fn build_mvhd(timescale: u32, duration: u32, next_track_id: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend(0u32.to_be_bytes());
    payload.extend(0u32.to_be_bytes());
    payload.extend(timescale.to_be_bytes());
    payload.extend(duration.to_be_bytes());
    payload.extend(0x0001_0000u32.to_be_bytes());
    payload.extend(0x0100u16.to_be_bytes());
    payload.extend(0u16.to_be_bytes());
    payload.extend([0u8; 8]);
    payload.extend(unity_matrix());
    payload.extend([0u8; 24]);
    payload.extend(next_track_id.to_be_bytes());
    make_box(b"mvhd", &payload)
}

fn build_trak(track: &Track, movie_timescale: u32, chunk_offset: u64) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    let movie_duration = scale_duration(track.duration, track.timescale, movie_timescale);
    payload.extend(build_tkhd(track, movie_duration));
    payload.extend(build_mdia(track, chunk_offset)?);
    Ok(make_box(b"trak", &payload))
}

fn build_tkhd(track: &Track, duration: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0x000007));
    payload.extend(0u32.to_be_bytes());
    payload.extend(0u32.to_be_bytes());
    payload.extend(track.track_id.to_be_bytes());
    payload.extend(0u32.to_be_bytes());
    payload.extend(duration.to_be_bytes());
    payload.extend([0u8; 8]);
    payload.extend(0u16.to_be_bytes());
    payload.extend(0u16.to_be_bytes());
    let volume = if track.handler == *b"soun" { 0x0100u16 } else { 0u16 };
    payload.extend(volume.to_be_bytes());
    payload.extend(0u16.to_be_bytes());
    payload.extend(unity_matrix());
    payload.extend((track.width << 16).to_be_bytes());
    payload.extend((track.height << 16).to_be_bytes());
    make_box(b"tkhd", &payload)
}

fn build_mdia(track: &Track, chunk_offset: u64) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    payload.extend(build_mdhd(track));
    payload.extend(build_hdlr(track.handler));
    payload.extend(build_minf(track, chunk_offset)?);
    Ok(make_box(b"mdia", &payload))
}

fn build_mdhd(track: &Track) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend(0u32.to_be_bytes());
    payload.extend(0u32.to_be_bytes());
    payload.extend(track.timescale.to_be_bytes());
    payload.extend((track.duration as u32).to_be_bytes());
    payload.extend(0x55c4u16.to_be_bytes());
    payload.extend(0u16.to_be_bytes());
    make_box(b"mdhd", &payload)
}

fn build_hdlr(handler: [u8; 4]) -> Vec<u8> {
    let name = if handler == *b"vide" { b"VideoHandler\0".as_slice() } else { b"SoundHandler\0".as_slice() };
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend(0u32.to_be_bytes());
    payload.extend(handler);
    payload.extend([0u8; 12]);
    payload.extend(name);
    make_box(b"hdlr", &payload)
}

fn build_minf(track: &Track, chunk_offset: u64) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    if track.handler == *b"vide" {
        payload.extend(make_full_box(b"vmhd", 0, 1, &[0u8; 8]));
    } else {
        payload.extend(make_full_box(b"smhd", 0, 0, &[0u8; 4]));
    }
    payload.extend(build_dinf());
    payload.extend(build_stbl(track, chunk_offset)?);
    Ok(make_box(b"minf", &payload))
}

fn build_dinf() -> Vec<u8> {
    let url = make_full_box(b"url ", 0, 1, &[]);
    let mut dref_payload = Vec::new();
    dref_payload.extend(full_box_header(0, 0));
    dref_payload.extend(1u32.to_be_bytes());
    dref_payload.extend(url);
    make_box(b"dinf", &make_box(b"dref", &dref_payload))
}

fn build_stbl(track: &Track, chunk_offset: u64) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    payload.extend(&track.stsd_box);
    payload.extend(build_stts(track));
    if track.samples.iter().any(|sample| sample.cts_offset != 0) {
        payload.extend(build_ctts(track)?);
    }
    payload.extend(build_stsc(track));
    payload.extend(build_stsz(track));
    payload.extend(build_stco(chunk_offset)?);
    if track.handler == *b"vide" {
        payload.extend(build_stss(track));
    }
    Ok(make_box(b"stbl", &payload))
}

fn build_stts(track: &Track) -> Vec<u8> {
    let mut entries: Vec<(u32, u32)> = Vec::new();
    for sample in &track.samples {
        if let Some((count, duration)) = entries.last_mut() {
            if *duration == sample.duration {
                *count += 1;
                continue;
            }
        }
        entries.push((1, sample.duration));
    }

    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend((entries.len() as u32).to_be_bytes());
    for (count, duration) in entries {
        payload.extend(count.to_be_bytes());
        payload.extend(duration.to_be_bytes());
    }
    make_box(b"stts", &payload)
}

fn build_ctts(track: &Track) -> Result<Vec<u8>, String> {
    let version = if track.samples.iter().any(|sample| sample.cts_offset < 0) { 1 } else { 0 };
    let mut entries: Vec<(u32, i32)> = Vec::new();
    for sample in &track.samples {
        if let Some((count, offset)) = entries.last_mut() {
            if *offset == sample.cts_offset {
                *count += 1;
                continue;
            }
        }
        entries.push((1, sample.cts_offset));
    }

    let mut payload = Vec::new();
    payload.extend(full_box_header(version, 0));
    payload.extend((entries.len() as u32).to_be_bytes());
    for (count, offset) in entries {
        payload.extend(count.to_be_bytes());
        if version == 0 && offset < 0 {
            return Err("ctts version 0 不支持负 offset".to_string());
        }
        payload.extend(offset.to_be_bytes());
    }
    Ok(make_box(b"ctts", &payload))
}

fn build_stsc(track: &Track) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend(1u32.to_be_bytes());
    payload.extend(1u32.to_be_bytes());
    payload.extend((track.samples.len() as u32).to_be_bytes());
    payload.extend(1u32.to_be_bytes());
    make_box(b"stsc", &payload)
}

fn build_stsz(track: &Track) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend(0u32.to_be_bytes());
    payload.extend((track.samples.len() as u32).to_be_bytes());
    for sample in &track.samples {
        payload.extend(sample.size.to_be_bytes());
    }
    make_box(b"stsz", &payload)
}

fn build_stco(chunk_offset: u64) -> Result<Vec<u8>, String> {
    if chunk_offset > u32::MAX as u64 {
        return Err("暂不支持超过 4GB 偏移的 MP4".to_string());
    }
    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend(1u32.to_be_bytes());
    payload.extend((chunk_offset as u32).to_be_bytes());
    Ok(make_box(b"stco", &payload))
}

fn build_stss(track: &Track) -> Vec<u8> {
    let sync_samples: Vec<u32> = track
        .samples
        .iter()
        .enumerate()
        .filter_map(|(index, sample)| sample.sync.then_some(index as u32 + 1))
        .collect();

    let mut payload = Vec::new();
    payload.extend(full_box_header(0, 0));
    payload.extend((sync_samples.len() as u32).to_be_bytes());
    for index in sync_samples {
        payload.extend(index.to_be_bytes());
    }
    make_box(b"stss", &payload)
}

fn find_following_mdat(source: &[u8], moof: BoxHeader) -> Option<usize> {
    let mut offset = moof.end();
    while let Some(header) = read_box(source, offset, source.len()) {
        if &header.typ == b"mdat" {
            return Some(header.content_start());
        }
        if &header.typ == b"moof" {
            return None;
        }
        offset = header.end();
    }
    None
}

fn find_path(source: &[u8], start: usize, end: usize, path: &[&[u8; 4]]) -> Option<BoxHeader> {
    if path.is_empty() {
        return None;
    }
    let header = find_box(source, start, end, path[0])?;
    if path.len() == 1 {
        Some(header)
    } else {
        find_path(source, header.content_start(), header.end(), &path[1..])
    }
}

fn find_box(source: &[u8], start: usize, end: usize, typ: &[u8; 4]) -> Option<BoxHeader> {
    let mut offset = start;
    while let Some(header) = read_box(source, offset, end) {
        if &header.typ == typ {
            return Some(header);
        }
        offset = header.end();
    }
    None
}

fn read_box(source: &[u8], offset: usize, limit: usize) -> Option<BoxHeader> {
    if offset + 8 > limit || offset + 8 > source.len() {
        return None;
    }

    let size32 = read_u32(&source[offset..offset + 4]);
    let typ = source[offset + 4..offset + 8].try_into().ok()?;
    let (size, header_size) = if size32 == 1 {
        if offset + 16 > limit || offset + 16 > source.len() {
            return None;
        }
        (read_u64(&source[offset + 8..offset + 16]) as usize, 16)
    } else if size32 == 0 {
        (limit - offset, 8)
    } else {
        (size32 as usize, 8)
    };

    if size < header_size || offset.checked_add(size)? > limit || offset + size > source.len() {
        return None;
    }

    Some(BoxHeader {
        typ,
        start: offset,
        header_size,
        size,
    })
}

fn read_handler_type(source: &[u8], hdlr: BoxHeader) -> Result<[u8; 4], String> {
    let data = &source[hdlr.content_start()..hdlr.end()];
    if data.len() < 12 {
        return Err("hdlr 太短".to_string());
    }
    Ok(data[8..12].try_into().unwrap())
}

fn read_tkhd(source: &[u8], tkhd: BoxHeader) -> Result<(u32, u32, u32), String> {
    let data = &source[tkhd.content_start()..tkhd.end()];
    if data.len() < 84 {
        return Err("tkhd 太短".to_string());
    }

    let version = data[0];
    let track_id_pos = if version == 1 { 20 } else { 12 };
    let track_id = read_u32_checked(data, track_id_pos, "tkhd track id")?;
    let width = read_u32(&data[data.len() - 8..data.len() - 4]) >> 16;
    let height = read_u32(&data[data.len() - 4..]) >> 16;
    Ok((track_id, width, height))
}

fn read_mdhd_timescale(source: &[u8], mdhd: BoxHeader) -> Result<u32, String> {
    let data = &source[mdhd.content_start()..mdhd.end()];
    if data.len() < 20 {
        return Err("mdhd 太短".to_string());
    }

    let version = data[0];
    let timescale_pos = if version == 1 { 20 } else { 12 };
    read_u32_checked(data, timescale_pos, "mdhd timescale")
}

fn read_trex(source: &[u8], moov: BoxHeader, track_id: u32) -> Option<TrexInfo> {
    let mvex = find_box(source, moov.content_start(), moov.end(), b"mvex")?;
    let mut offset = mvex.content_start();
    while let Some(trex) = read_box(source, offset, mvex.end()) {
        offset = trex.end();
        if &trex.typ != b"trex" {
            continue;
        }

        let data = &source[trex.content_start()..trex.end()];
        if data.len() < 24 {
            continue;
        }

        if read_u32(&data[4..8]) == track_id {
            let default_duration = read_u32(&data[12..16]);
            let default_size = read_u32(&data[16..20]);
            let default_flags = read_u32(&data[20..24]);
            return Some(TrexInfo {
                default_duration: (default_duration != 0).then_some(default_duration),
                default_size: (default_size != 0).then_some(default_size),
                default_flags: Some(default_flags),
            });
        }
    }

    None
}

fn make_box(typ: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = payload.len() + 8;
    let mut out = Vec::with_capacity(size);
    out.extend((size as u32).to_be_bytes());
    out.extend(typ);
    out.extend(payload);
    out
}

fn make_full_box(typ: &[u8; 4], version: u8, flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut full_payload = full_box_header(version, flags);
    full_payload.extend(payload);
    make_box(typ, &full_payload)
}

fn full_box_header(version: u8, flags: u32) -> Vec<u8> {
    vec![
        version,
        ((flags >> 16) & 0xff) as u8,
        ((flags >> 8) & 0xff) as u8,
        (flags & 0xff) as u8,
    ]
}

fn write_box_header(file: &mut fs::File, typ: &[u8; 4], size: u32) -> Result<(), String> {
    file.write_all(&size.to_be_bytes()).map_err(|e| format!("写入 box size 失败: {}", e))?;
    file.write_all(typ).map_err(|e| format!("写入 box type 失败: {}", e))?;
    Ok(())
}

fn scale_duration(duration: u64, from_timescale: u32, to_timescale: u32) -> u32 {
    ((duration.saturating_mul(to_timescale as u64)) / from_timescale as u64).min(u32::MAX as u64) as u32
}

fn unity_matrix() -> [u8; 36] {
    [
        0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x40, 0x00, 0x00, 0x00,
    ]
}

fn read_u24(data: &[u8]) -> u32 {
    ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32
}

fn read_u32(data: &[u8]) -> u32 {
    u32::from_be_bytes(data[0..4].try_into().unwrap())
}

fn read_u64(data: &[u8]) -> u64 {
    u64::from_be_bytes(data[0..8].try_into().unwrap())
}

fn read_u32_checked(data: &[u8], pos: usize, label: &str) -> Result<u32, String> {
    if pos + 4 > data.len() {
        return Err(format!("{} 越界", label));
    }
    Ok(read_u32(&data[pos..pos + 4]))
}

fn read_i32_checked(data: &[u8], pos: usize, label: &str) -> Result<i32, String> {
    if pos + 4 > data.len() {
        return Err(format!("{} 越界", label));
    }
    Ok(i32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()))
}
